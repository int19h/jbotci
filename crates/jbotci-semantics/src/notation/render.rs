//! The `smusni` graph walk: a faithful Rust port of the Python oracle's `lean3`
//! rendering path (`render_v5.py` at commit `28c7d5f`; `lean3` is the
//! research repo's historical name for what ships as the `smusni` profile).
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
//! Only the `smusni` profile is realised here: the frozen `smusni` option set
//! (`opt_compact_dimensions`, `opt_bracket_keys`, `opt_collapse_notcomputed`,
//! `opt_terse_labels`, `opt_glyph_formulas`, `opt_braces`, `opt_short_ids`,
//! `opt_dense_decls`, all on; `opt_provenance_off` following the runtime
//! `provenance` toggle). The experiment-only options that `smusni` leaves off
//! (nav-index, scope-paths, inline-lambda, colocated-defs, discourse-order,
//! infix-implication, inline-introductions, content-ids) are intentionally not
//! ported — they are not part of `smusni` and are unreachable through this API.

use std::collections::BTreeMap;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use serde_json::{Map, Value};

use crate::model::SemanticGraph;

use super::writer::Writer;

/// `smusni` render configuration. `provenance` is the one runtime toggle
/// (`--provenance` / `opt_provenance_off` off): when set, source spans/text are
/// rendered; otherwise the profile renders semantic content only.
// `#[invariant(true)]`: an audited no-op — a single `bool` toggle, so every
// value is a valid configuration; the field type already expresses the whole
// domain.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SmusniConfig {
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
    ("question", "qu"),
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
    // Fallback for a key with no trailing decimal run. Real this-build graph
    // keys always end in their counter digit, so this path is unexercised by
    // the corpus; the debug assertion documents and guards its one invariant
    // (a non-empty key yields a non-empty id component) since no test reaches it
    // (round-1 review, kimi 10).
    let sanitized = sanitize_non_word(key);
    debug_assert!(
        key.is_empty() || !sanitized.is_empty(),
        "key_number fallback produced an empty component for non-empty key `{key}`"
    );
    sanitized
}

/// Replace every maximal run of non-word characters with a single `_`, where
/// "word" is the ASCII class `[A-Za-z0-9_]`. The oracle uses Python
/// `re.sub(r"\W+", "_", s)`; note Python's `\W` is Unicode-aware by default (it
/// treats non-ASCII letters as word chars), so the two would differ on a key
/// containing non-ASCII letters — which cannot arise here: this runs only on the
/// `key_number` fallback for a key with no trailing digit, and every graph key
/// is ASCII (`<sort-or-kind>:<counter>`). Documented precisely rather than
/// claimed byte-equivalent on all inputs (round-1 review, kimi 9).
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
/// the `opt_short_ids` (N8c) scheme with a collision-disambiguation loop.
///
/// The base ID is `<prefix><num>` (or `<kind>_<num>` for an unrecognized kind).
/// If that base is already taken, it is disambiguated by appending the source
/// key (with `:`/`/`→`_`); and — round-1 review (kimi 3): the single-step
/// fallback could itself collide — if the disambiguated form is *also* taken
/// (two distinct keys can sanitise equal, e.g. `a:b` and `a/b`), a numeric
/// suffix is bumped until the ID is unique. Injectivity is proven by the
/// postcondition (every graph key maps to a distinct ID), so no two objects can
/// silently share an ID. Lockstep with the oracle's identical loop.
#[requires(true)]
#[ensures(ret.len() == order.len())]
#[ensures(ret.values().collect::<std::collections::BTreeSet<&String>>().len() == ret.len())]
fn build_id_map(order: &[String], objects: &BTreeMap<String, Value>) -> BTreeMap<String, String> {
    let mut id_map = BTreeMap::new();
    let mut used = std::collections::BTreeSet::new();
    for key in order {
        let obj = &objects[key];
        let kind = id_kind_for(obj);
        let num = key_number(key);
        let base = match short_prefix(kind) {
            Some(prefix) => format!("{prefix}{num}"),
            None => format!("{kind}_{num}"),
        };
        let mut vid = base.clone();
        if used.contains(&vid) {
            let disambiguated = format!("{base}_{}", sanitize_colon_slash(key));
            vid = disambiguated.clone();
            let mut n = 2;
            while used.contains(&vid) {
                vid = format!("{disambiguated}_{n}");
                n += 1;
            }
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

/// First character uppercased, the rest lowercased — matching Python
/// `str.capitalize` on the ASCII sort names this is applied to (`entity` →
/// `Entity`). Both use Unicode case mapping, so they coincide on ASCII; on
/// exotic Unicode input Python's titlecase-first-codepoint rule could differ,
/// but `sort` values are ASCII lowercase words (round-1 review, kimi 9).
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

/// Threads the object lookup, id map, and the one `smusni` runtime toggle through
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
        self.id_map.get(key).map(String::as_str).unwrap_or_else(|| {
            panic!("graph pointer `{key}` has no generated id (malformed graph)")
        })
    }

    /// The generated ID for a graph key held in a JSON string value.
    #[requires(true)]
    #[ensures(true)]
    fn id_of(&self, value: &Value) -> String {
        self.id(pointer_key(value)).to_string()
    }

    /// The object a graph key points at. Absence means a dangling pointer, i.e.
    /// a violated referential-integrity invariant, which fails loudly.
    #[requires(true)]
    #[ensures(true)]
    fn object(&self, key: &str) -> &Value {
        self.objects.get(key).unwrap_or_else(|| {
            panic!("graph pointer `{key}` resolves to no object (contract violated)")
        })
    }
}

/// A graph-pointer string value, strictly (a pointer field must hold a string).
#[requires(true)]
#[ensures(true)]
fn pointer_key(value: &Value) -> &str {
    value.as_str().unwrap_or_else(|| {
        panic!("expected a graph-pointer string, found {value} (contract violated)")
    })
}

/// Read an optional string field (absent → `None`).
#[requires(true)]
#[ensures(true)]
fn field_str<'a>(obj: &'a Value, key: &str) -> Option<&'a str> {
    obj.get(key).and_then(Value::as_str)
}

/// Read a REQUIRED string field. A valid `lojban-semantics-json-1` graph built
/// by *this* jbotci build always carries it (the model's own type invariants
/// make the field mandatory); its absence means the graph contract was violated
/// — schema drift or a hand-forged graph — which fails loudly here with the
/// field name rather than silently emitting blank notation (round-1 review:
/// strict accessors over `unwrap_or("")` masking).
#[requires(true)]
#[ensures(true)]
fn req_str<'a>(obj: &'a Value, key: &str) -> &'a str {
    field_str(obj, key).unwrap_or_else(|| {
        panic!("required field `{key}` absent (lojban-semantics-json-1 contract violated)")
    })
}

/// Read a REQUIRED field as a raw `Value` (for a mandatory pointer edge). Same
/// loud-on-violation contract as [`req_str`].
#[requires(true)]
#[ensures(true)]
fn req_val<'a>(obj: &'a Value, key: &str) -> &'a Value {
    obj.get(key).unwrap_or_else(|| {
        panic!("required field `{key}` absent (lojban-semantics-json-1 contract violated)")
    })
}

/// A JSON number rendered bare (integers as `190`, no quotes), for VALUE/ARITY.
#[requires(true)]
#[ensures(true)]
fn number_str(value: &Value) -> String {
    value.to_string()
}

/// The public render entry: walk `graph` and produce its `smusni` notation.
///
/// # Contract
///
/// `graph` must be a valid `lojban-semantics-json-1` graph produced by *this*
/// jbotci build: its `SemanticGraph` type invariants already guarantee
/// referential integrity (every pointer resolves — `semantic_graph_references_are_defined`)
/// and that required fields are populated. The renderer relies on those
/// invariants (the strict accessors [`req_str`]/[`req_val`]/[`Ctx::id`] fail
/// loudly if they are ever violated rather than degrading to blank notation);
/// it never fabricates or guesses missing structure. A genuinely unknown object
/// `type` is the one open case and is handled explicitly by the `UNKNOWN … NOT
/// COMPUTED` path, never a panic.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(ret.ends_with('\n'))]
pub fn render_smusni(graph: &SemanticGraph, config: SmusniConfig) -> String {
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
        "question" => render_question(w, ctx, key, obj),
        other => {
            let vid = ctx.id(key).to_string();
            // The raw `type` is quoted (round-1 review, kimi 10): an unknown or
            // untrusted type could carry notation metacharacters (`;`, `{`, …)
            // that would otherwise break the surrounding NOT COMPUTED marker.
            let marker = format!("renderer-support({})", quote(other));
            w.declaration("UNKNOWN", &vid, None, true, |w| {
                w.field("NOT COMPUTED", &marker);
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
        render_source_fields(w, src);
    });
}

/// The coordinates inside a `PROVENANCE` block. Keeping the block wrapper
/// separate lets nested modal arguments put their lexical introducer alongside
/// the source coordinates without inventing a content-level cmavo field.
#[requires(true)]
#[ensures(true)]
fn render_source_fields(w: &mut Writer, source: &Value) {
    if let Some(span) = source.get("span").filter(|span| !span.is_null()) {
        if let (Some(start), Some(end)) = (span.get("byteStart"), span.get("byteEnd")) {
            w.field(
                "BYTE SPAN",
                &format!("{}..{}", number_str(start), number_str(end)),
            );
        }
    }
    if let Some(text) = field_str(source, "text") {
        w.field("TEXT", &quote(text));
    }
    if let Some(construct) = field_str(source, "construct") {
        w.field("CONSTRUCT", &quote(rendered_source_construct(construct)));
    }
}

/// JSON source constructs retain the versioned model's historical terminology,
/// but the human-facing notation uses neutral "tag" wording. This is an exact
/// vocabulary translation, not a substring rewrite, so unknown constructs stay
/// visible verbatim.
#[requires(!construct.is_empty())]
#[ensures(!ret.is_empty())]
fn rendered_source_construct(construct: &str) -> &str {
    match construct {
        "modal-argument" => "tagged-argument",
        "modal-indicator" => "tagged-indicator",
        "modal-fragment" => "tagged-fragment",
        "tense-modal-fragment" => "tense-tag-fragment",
        "modal-branch-formula" => "tag-branch-formula",
        "modal-connection-formula" => "tag-connection-formula",
        _ => construct,
    }
}

/// Modal introducers (`va'o`, `se pi'o`, `fi'o ...`) are surface provenance,
/// not semantic notation keywords. They are therefore emitted only in the
/// opt-in provenance profile, in the same block as the modal's source span.
#[requires(true)]
#[ensures(true)]
fn render_modal_provenance(w: &mut Writer, ctx: &Ctx, modal_argument: &Value) {
    if !ctx.provenance {
        return;
    }
    w.heading("PROVENANCE", |w| {
        w.field(
            "INTRODUCED BY",
            &lexical(req_str(modal_argument, "introducedBy")),
        );
        if let Some(source) = modal_argument
            .get("source")
            .filter(|source| !source.is_null())
        {
            render_source_fields(w, source);
        }
    });
}

/// Emit a compact one-line `label` as a plain `entry`, EXCEPT when provenance is
/// on AND the nested value struct `obj` carries its own `source` — then `label`
/// becomes a heading whose body is that struct's nested PROVENANCE block
/// (Phase-B Amendment 2, round-2 review). This keeps the compact-entry form
/// byte-identical whenever provenance is off or the struct has no source, so the
/// provenance delta is confined to the newly rendered nested sources; a
/// sourceless sibling in the same collection stays a plain entry. Used for the
/// value structs the renderer traverses whose `source` the flat top-level
/// PROVENANCE pass never reaches — predication `ArgumentValue` fillers and
/// descriptor-attached `RelativeClause`s (`AssignedName`, already a heading,
/// calls [`render_source`] directly). Lockstep with the oracle's
/// `render_with_optional_source`.
#[requires(true)]
#[ensures(true)]
fn render_with_optional_source(w: &mut Writer, ctx: &Ctx, label: &str, obj: &Value) {
    if ctx.provenance && obj.get("source").is_some_and(|source| !source.is_null()) {
        w.heading(label, |w| render_source(w, ctx, obj));
    } else {
        w.entry(label);
    }
}

/// A `RELATIVE CLAUSES` collection, shared by descriptor-attached clauses and
/// (Amendment 3, round-3 review) predication-argument-attached clauses. Each
/// clause is the compact `<KIND> <id>` entry, or — under provenance with a
/// source — a heading carrying its own nested source ([`render_with_optional_source`]).
/// Argument-attached clauses were previously dropped: the semantics builder
/// attaches them only to `ArgumentValue.relative_clauses`
/// (`attach_generated_relative_clauses_to_argument`), with no invariant
/// duplicating them as a rendered restriction, so rendering them here is the
/// field-completeness fix, not a redundant echo. `kind`/`body` are
/// required-when-reached (`body` via strict [`req_val`]).
#[requires(true)]
#[ensures(true)]
fn render_relative_clauses(w: &mut Writer, ctx: &Ctx, clauses: &[Value]) {
    w.collection("RELATIVE CLAUSES", |w| {
        for clause in clauses {
            let text = format!(
                "{} {}",
                enum_render(req_str(clause, "kind")),
                ctx.id_of(req_val(clause, "body"))
            );
            render_with_optional_source(w, ctx, &text, clause);
        }
    });
}

/// `scopeDependence`: a `SCOPE DEPENDENCE: <kind>;` scalar, or a heading with a
/// `MAY DEPEND ON { ... }` collection when the referent depends on binders.
#[requires(true)]
#[ensures(true)]
fn render_scope_dependence(w: &mut Writer, ctx: &Ctx, sd: &Value) {
    let kind = req_str(sd, "kind");
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
    let kind = req_str(item, "kind");
    let introduced_by = req_str(item, "introducedBy");
    let mut text = format!(
        "{} INTRODUCED BY {}",
        enum_render(kind),
        lexical(introduced_by)
    );
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

/// One `intervalModifiers` entry (`IntervalModifier`, a `{kind, value}` tagged
/// union). Adjudicated rendered (content-complete doctrine; round-1 review) —
/// previously dropped by renderer and oracle alike. It is NOT provably redundant
/// with the `aspect`/`recurrence` dimension-record fields (ordering and
/// per-modifier distinctions are uncaptured there), so it renders in full,
/// reusing the same value renderers as its dimension twins so the two forms
/// cannot drift; any other kind, or an aspect modifier without a `contour`, is
/// flagged rather than guessed at.
#[requires(true)]
#[ensures(!ret.is_empty())]
fn interval_modifier_text(ctx: &Ctx, modifier: &Value) -> String {
    let kind = req_str(modifier, "kind");
    let value = modifier.get("value").cloned().unwrap_or(Value::Null);
    match kind {
        "aspect" => match field_str(&value, "contour") {
            Some(contour) => format!("{} {}", enum_render(kind), enum_render(contour)),
            None => "NOT COMPUTED: interval-modifier-shape(aspect)".to_string(),
        },
        "recurrence" => format!(
            "{} {}",
            enum_render(kind),
            recurrence_item_text(ctx, &value)
        ),
        other => format!("NOT COMPUTED: interval-modifier-shape({other})"),
    }
}

/// The `intervalModifiers` collection on an eventuality reference. Complete-or-
/// absent, like every other optional collection here.
#[requires(true)]
#[ensures(true)]
fn render_interval_modifiers(w: &mut Writer, ctx: &Ctx, obj: &Value) {
    if let Some(mods) = obj
        .get("intervalModifiers")
        .and_then(Value::as_array)
        .filter(|m| !m.is_empty())
    {
        w.collection("INTERVAL MODIFIERS", |w| {
            for modifier in mods {
                w.entry(&interval_modifier_text(ctx, modifier));
            }
        });
    }
}

/// The `assignedNames` collection on a referent (`AssignedName`: the goi/cei-
/// assigned cmavo, the naming word, and the assignment marker). Adjudicated
/// rendered (content-complete doctrine; round-1 review) — previously dropped by
/// renderer and oracle alike. Each name is a small structured record, so it
/// renders as a heading group (like DESCRIPTOR / DEICTIC GROUND) inside the
/// collection: WORD/INTRODUCED BY are Lojban cmavo, NAME is the assigned label
/// as witness text (quoted, matching DESCRIPTOR's own NAME field). Per-name
/// `source` provenance obeys the same profile gate as every other source and is
/// not rendered here.
#[requires(true)]
#[ensures(true)]
fn render_assigned_names(w: &mut Writer, ctx: &Ctx, obj: &Value) {
    if let Some(names) = obj
        .get("assignedNames")
        .and_then(Value::as_array)
        .filter(|n| !n.is_empty())
    {
        w.collection("ASSIGNED NAMES", |w| {
            for name in names {
                w.heading("ASSIGNED NAME", |w| {
                    w.field("WORD", &lexical(req_str(name, "word")));
                    w.field("NAME", &quote(req_str(name, "name")));
                    w.field("INTRODUCED BY", &lexical(req_str(name, "introducedBy")));
                    // Amendment 2: the AssignedName's own `source` renders under
                    // provenance (render_source is a no-op when provenance off).
                    render_source(w, ctx, name);
                });
            }
        });
    }
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
            let relation = enum_render(req_str(t, "relation"));
            let anchor = ctx.id_of(req_val(t, "anchor"));
            pairs.push((
                "time".to_string(),
                format!("{{ relation = {relation}, anchor = {anchor} }}"),
            ));
        }
        None => pairs.push(("time".to_string(), "UNSPECIFIED".to_string())),
    }

    match obj.get("actuality").filter(|a| !a.is_null()) {
        Some(a) => pairs.push(("actuality".to_string(), enum_render(req_str(a, "kind")))),
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

    match obj
        .get("recurrence")
        .and_then(Value::as_array)
        .filter(|r| !r.is_empty())
    {
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
    let kind = req_str(d, "kind");
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
        if let Some(clauses) = d
            .get("relativeClauses")
            .and_then(Value::as_array)
            .filter(|c| !c.is_empty())
        {
            render_relative_clauses(w, ctx, clauses);
        }
    });
}

/// §6.3 operand mode: `VALUE <id>` for a bound singular value (`parameter`
/// target), `REFERENCE DENOTATION <id>` for a full reference denotation.
#[requires(true)]
#[ensures(!ret.is_empty())]
fn operand_ref(ctx: &Ctx, value: &Value) -> String {
    let val = value.as_str().unwrap_or_else(|| {
        panic!("operand filler must be a graph-pointer string; found non-string JSON {value} (lojban-semantics-json-1 contract violated)")
    });
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

/// One `ArgumentValue` in an order-bearing callable-place map. This is shared
/// by ordinary numbered predication places and the places nested below a modal
/// predicate keyword. Occurrence-owned relative clauses and provenance retain
/// exactly the same rendering in both contexts.
#[requires(!label.is_empty())]
#[ensures(true)]
fn render_argument_value_entry(w: &mut Writer, ctx: &Ctx, label: &str, argument: &Value) {
    let operand = operand_ref(ctx, req_val(argument, "value"));
    let label = format!("{label} {operand}");
    let relative_clauses = argument
        .get("relativeClauses")
        .and_then(Value::as_array)
        .filter(|clauses| !clauses.is_empty());
    let show_source = ctx.provenance
        && argument
            .get("source")
            .is_some_and(|source| !source.is_null());
    if relative_clauses.is_some() || show_source {
        w.heading(&label, |w| {
            if let Some(clauses) = relative_clauses {
                render_relative_clauses(w, ctx, clauses);
            }
            render_source(w, ctx, argument);
        });
    } else {
        w.entry(&label);
    }
}

/// Emit the entries of a canonical callable-place map without adding an
/// enclosing `ARGS`. Callers use this for both the host predication's numbered
/// places and a modal predicate's recursively nested places.
#[requires(!arguments.is_empty())]
#[ensures(true)]
fn render_argument_entries(w: &mut Writer, ctx: &Ctx, arguments: &Map<String, Value>) {
    let mut keys: Vec<&String> = arguments.keys().collect();
    keys.sort_by_key(|key| place_number(key));
    for key in keys {
        let place = place_number(key);
        assert!(
            place > 0 && key == &format!("x{place}"),
            "argument place `{key}` is not a canonical xN key (contract violated)"
        );
        render_argument_value_entry(w, ctx, &format!("[{place}]:"), &arguments[key]);
    }
}

/// A scalar-negation value nested in a modal argument.
#[requires(true)]
#[ensures(true)]
fn render_scalar_negation(w: &mut Writer, ctx: &Ctx, scalar_negation: &Value) {
    w.heading("SCALAR NEGATION", |w| {
        w.field("KIND", &enum_render(req_str(scalar_negation, "kind")));
        w.field(
            "INTRODUCED BY",
            &lexical(req_str(scalar_negation, "introducedBy")),
        );
        if let Some(scale) = scalar_negation.get("scale") {
            w.field("SCALE", &ctx.id_of(scale));
        }
        if let Some(argument_scope) = scalar_negation
            .get("argumentScope")
            .and_then(Value::as_array)
            .filter(|scope| !scope.is_empty())
        {
            w.ordered("ARGUMENT SCOPE", |w| {
                for place in argument_scope {
                    let key = pointer_key(place);
                    let number = place_number(key);
                    assert!(
                        number > 0 && key == format!("x{number}"),
                        "scalar-negation argument scope `{key}` is not a canonical xN key \
                         (contract violated)"
                    );
                    w.entry(&format!("[{number}]"));
                }
            });
        }
    });
}

/// One displayed-content modifier nested in a modal argument.
#[requires(true)]
#[ensures(true)]
fn render_displayed_content_modifier(w: &mut Writer, ctx: &Ctx, modifier: &Value) {
    w.field("RELATION", &lexical(req_str(modifier, "relation")));
    if let Some(family) = field_str(modifier, "family") {
        w.field("FAMILY", &enum_render(family));
    }
    if let Some(polarity) = field_str(modifier, "polarity") {
        w.field("POLARITY", &enum_render(polarity));
    }
    if let Some(intensity) = field_str(modifier, "intensity") {
        w.field("INTENSITY", &lexical(intensity));
    }
    if let Some(assertion_effect) = field_str(modifier, "assertionEffect") {
        w.field("ASSERTION EFFECT", &enum_render(assertion_effect));
    }
    render_source(w, ctx, modifier);
}

/// Content fields shared by relation-keyed and formula-keyed modal entries.
#[requires(true)]
#[ensures(true)]
fn render_modal_entry_metadata(w: &mut Writer, ctx: &Ctx, modal_argument: &Value) {
    if let Some(negation) = modal_argument
        .get("negation")
        .filter(|value| !value.is_null())
    {
        w.heading("NEGATION", |w| {
            w.field("KIND", &enum_render(req_str(negation, "kind")));
            w.field("INTRODUCED BY", &lexical(req_str(negation, "introducedBy")));
        });
    }
    if let Some(scalar_negation) = modal_argument
        .get("scalarNegation")
        .filter(|value| !value.is_null())
    {
        render_scalar_negation(w, ctx, scalar_negation);
    }
    if let Some(modifiers) = modal_argument
        .get("modifiers")
        .and_then(Value::as_array)
        .filter(|modifiers| !modifiers.is_empty())
    {
        w.ordered("MODIFIERS", |w| {
            for (index, modifier) in modifiers.iter().enumerate() {
                w.heading(&format!("[{}]:", index + 1), |w| {
                    render_displayed_content_modifier(w, ctx, modifier);
                });
            }
        });
    }
    render_modal_provenance(w, ctx, modal_argument);
}

/// Whether a formula-keyed modal needs a block rather than the compact
/// `[formula]: REFERENCE DENOTATION component;` entry.
#[requires(true)]
#[ensures(true)]
fn formula_modal_has_metadata(ctx: &Ctx, modal_argument: &Value) -> bool {
    modal_argument
        .get("negation")
        .is_some_and(|value| !value.is_null())
        || modal_argument
            .get("scalarNegation")
            .is_some_and(|value| !value.is_null())
        || modal_argument
            .get("modifiers")
            .and_then(Value::as_array)
            .is_some_and(|modifiers| !modifiers.is_empty())
        || ctx.provenance
}

/// Render one modal as a keyword-indexed `ARGS` entry. A relation modal uses
/// the desugared predicate word as its key and recursively renders that
/// predicate's numbered places. An ad-hoc `fi'o` modal uses its formula ID as
/// the key and the host component as the value.
#[requires(true)]
#[ensures(true)]
fn render_modal_argument_entry(w: &mut Writer, ctx: &Ctx, modal_argument: &Value) {
    if let Some(relation) = field_str(modal_argument, "relation") {
        let arguments = req_val(modal_argument, "arguments")
            .as_object()
            .unwrap_or_else(|| {
                panic!("modal `arguments` must be a JSON object (contract violated)")
            });
        assert!(
            !arguments.is_empty(),
            "relation modal must have at least one argument (contract violated)"
        );
        w.ordered(&format!("[{relation}]:"), |w| {
            render_argument_entries(w, ctx, arguments);
            if let Some(component) = modal_argument.get("component") {
                w.field("COMPONENT", &ctx.id_of(component));
            }
            render_modal_entry_metadata(w, ctx, modal_argument);
        });
        return;
    }

    let body = ctx.id_of(req_val(modal_argument, "body"));
    let component = ctx.id_of(req_val(modal_argument, "component"));
    let label = format!("[{body}]: REFERENCE DENOTATION {component}");
    if formula_modal_has_metadata(ctx, modal_argument) {
        w.heading(&label, |w| {
            render_modal_entry_metadata(w, ctx, modal_argument);
        });
    } else {
        w.entry(&label);
    }
}

/// Host numbered arguments followed by modal keyword entries, all inside the
/// single existing `ARGS` sequence. Modal array order is canonical JSON
/// document order and is preserved exactly.
#[requires(true)]
#[ensures(true)]
fn render_arguments(w: &mut Writer, ctx: &Ctx, obj: &Value) {
    let arguments = obj
        .get("arguments")
        .and_then(Value::as_object)
        .filter(|arguments| !arguments.is_empty());
    let modal_arguments = obj
        .get("modalArguments")
        .and_then(Value::as_array)
        .filter(|arguments| !arguments.is_empty());
    if arguments.is_none() && modal_arguments.is_none() {
        return;
    }
    w.ordered("ARGS", |w| {
        if let Some(arguments) = arguments {
            render_argument_entries(w, ctx, arguments);
        }
        if let Some(modal_arguments) = modal_arguments {
            for modal_argument in modal_arguments {
                render_modal_argument_entry(w, ctx, modal_argument);
            }
        }
    });
}

#[requires(true)]
#[ensures(true)]
fn render_utterance(w: &mut Writer, ctx: &Ctx, key: &str, obj: &Value) {
    let vid = ctx.id(key).to_string();
    w.declaration("UTTERANCE", &vid, None, true, |w| {
        w.field("FORCE", &enum_render(req_str(obj, "force")));
        w.field("SPEAKER", &ctx.id_of(req_val(obj, "speaker")));
        w.field("AUDIENCE", &ctx.id_of(req_val(obj, "audience")));
        w.field("EVENTUALITY", &ctx.id_of(req_val(obj, "eventuality")));
        if let Some(content) = obj.get("content") {
            w.field("CONTENT", &ctx.id_of(content));
        }
        if let Some(vocative_kind) = field_str(obj, "vocativeKind") {
            w.field("VOCATIVE KIND", &enum_render(vocative_kind));
        }
        if let Some(dg) = obj.get("deicticGround").filter(|d| !d.is_null()) {
            w.heading("DEICTIC GROUND", |w| {
                w.field("TIME", &ctx.id_of(req_val(dg, "time")));
                w.field("PLACE", &ctx.id_of(req_val(dg, "place")));
            });
        }
        if let Some(asides) = obj
            .get("asides")
            .and_then(Value::as_array)
            .filter(|a| !a.is_empty())
        {
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
        // A predication's relation (`PredicationRelation`) is either a lexical
        // relation word (`relation`) or — for a relation-question or
        // relation-variable predication (`mo`, `bu'a`) — a bound relation
        // parameter (`relationParameter`, a pointer to a `parameter` object
        // whose own PARAMETER declaration carries ROLE: relation question /
        // relation variable). Exactly one field is present. A relation
        // parameter is referenced by the same `VALUE <id>` marker every other
        // parameter reference uses (§6.3 operand fillers, argument questions),
        // so the relation slot reads as a bound value and the question/variable
        // semantics live on its PARAMETER declaration — mirroring how a `ma`
        // argument-question already surfaces as `VALUE <id>` in ARGS. Neither
        // field present is a genuine `lojban-semantics-json-1` contract
        // violation and still fails loudly via `req_val`.
        if let Some(relation) = field_str(obj, "relation") {
            w.field("RELATION", &lexical(relation));
        } else {
            let parameter = ctx.id_of(req_val(obj, "relationParameter"));
            w.field("RELATION", &format!("VALUE {parameter}"));
        }
        if let Some(eventuality) = obj.get("eventuality") {
            w.field("EVENTUALITY", &ctx.id_of(eventuality));
        }
        render_arguments(w, ctx, obj);
        w.field("MODE", &enum_render(req_str(obj, "mode")));
        if let Some(place_questions) = obj
            .get("placeQuestions")
            .and_then(Value::as_array)
            .filter(|questions| !questions.is_empty())
        {
            render_place_questions(w, ctx, place_questions);
        }
        // `tanruLink` (`TanruLink`): a tanru's head/modifier structural link and
        // its synthesized relation label. Adjudicated rendered (content-complete
        // doctrine; round-1 review) — previously dropped by renderer and oracle
        // alike. HEAD is a predication id, MODIFIER an argument filler id, and
        // the relation label is synthesized Lojban tanru vocabulary.
        if let Some(tanru_link) = obj.get("tanruLink").filter(|t| !t.is_null()) {
            w.heading("TANRU LINK", |w| {
                w.field("HEAD", &ctx.id_of(req_val(tanru_link, "head")));
                w.field("MODIFIER", &ctx.id_of(req_val(tanru_link, "modifier")));
                w.field(
                    "RELATION LABEL",
                    &lexical(req_str(tanru_link, "relationLabel")),
                );
            });
        }
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

/// A predication's `placeQuestions` bindings. The outer vector is order-bearing,
/// so each binding receives the same one-based bracket key used by `ARGS`.
/// `candidatePlaces` is likewise serialized as ordered `xN` keys; it renders as
/// bracketed place numbers so the notation preserves both identity and order
/// without copying the JSON spelling.
#[requires(!questions.is_empty())]
#[ensures(true)]
fn render_place_questions(w: &mut Writer, ctx: &Ctx, questions: &[Value]) {
    w.ordered("PLACE QUESTIONS", |w| {
        for (index, question) in questions.iter().enumerate() {
            w.heading(&format!("[{}]:", index + 1), |w| {
                w.field("PARAMETER", &ctx.id_of(req_val(question, "parameter")));
                let argument = req_val(question, "argument");
                let operand = operand_ref(ctx, req_val(argument, "value"));
                let relative_clauses = argument
                    .get("relativeClauses")
                    .and_then(Value::as_array)
                    .filter(|clauses| !clauses.is_empty());
                let show_source = ctx.provenance
                    && argument
                        .get("source")
                        .is_some_and(|source| !source.is_null());
                if relative_clauses.is_some() || show_source {
                    w.heading(&format!("ARGUMENT: {operand}"), |w| {
                        if let Some(clauses) = relative_clauses {
                            render_relative_clauses(w, ctx, clauses);
                        }
                        render_source(w, ctx, argument);
                    });
                } else {
                    w.field("ARGUMENT", &operand);
                }
                let candidate_places = req_val(question, "candidatePlaces")
                    .as_array()
                    .filter(|places| !places.is_empty())
                    .unwrap_or_else(|| {
                        panic!(
                            "place-question `candidatePlaces` must be a non-empty array \
                             (contract violated)"
                        )
                    });
                w.ordered("CANDIDATE PLACES", |w| {
                    for place in candidate_places {
                        let place = pointer_key(place);
                        let number = place_number(place);
                        assert!(
                            number > 0 && place == format!("x{number}"),
                            "candidate place `{place}` is not a canonical xN key \
                             (contract violated)"
                        );
                        w.entry(&format!("[{number}]"));
                    }
                });
                render_source(w, ctx, question);
            });
        }
    });
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
    w.field("CONNECTIVE SOURCE", &lexical(req_str(connector, "source")));
    if full {
        let locus = match req_str(connector, "locus") {
            "modal" => "tag",
            locus => locus,
        };
        w.field("LOCUS", &enum_render(locus));
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
    let op = req_str(obj, "operator");
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
        if let Some(be) = obj
            .get("boundEventualities")
            .and_then(Value::as_array)
            .filter(|b| !b.is_empty())
        {
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
            // A quotation sign always carries a `quotation` object with a
            // `mode` (model invariant); both are required-when-reached.
            let q = req_val(obj, "quotation");
            let mode = req_str(q, "mode");
            w.field("MODE", &enum_render(mode));
            if mode == "parsed" && q.get("utterance").is_some() {
                w.field("QUOTED UTTERANCE", &ctx.id_of(req_val(q, "utterance")));
            } else if mode == "opaque" && q.get("text").is_some() {
                w.field("QUOTED TEXT", &quote(req_str(q, "text")));
                if let Some(delimiter) = field_str(q, "delimiter") {
                    w.field("DELIMITER", &quote(delimiter));
                }
            } else {
                w.field("NOT COMPUTED", &format!("quotation-shape({mode})"));
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
        w.annotate("DENOTES VALUES OF SORT", &title_sort(req_str(obj, "sort")));
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
        if let Some(deictic) = obj.get("deicticReference") {
            w.heading("DEICTIC REFERENCE", |w| {
                w.field("PROXIMITY", &enum_render(req_str(deictic, "proximity")));
                w.field("GROUND", &ctx.id_of(req_val(deictic, "ground")));
            });
        }
        if let Some(personal) = obj.get("personalMassMembership") {
            w.heading("PERSONAL MASS MEMBERSHIP", |w| {
                for (label, field) in [("SPEAKER", "speaker"), ("AUDIENCE", "audience")] {
                    let participant = req_val(personal, field);
                    w.field(
                        label,
                        &format!(
                            "{} {}",
                            enum_render(req_str(participant, "membership")),
                            ctx.id_of(req_val(participant, "referent"))
                        ),
                    );
                }
                if let Some(others) = personal.get("others") {
                    w.field("OTHERS", &ctx.id_of(others));
                }
            });
        }
        if let Some(generated) = obj.get("generatedReferent") {
            w.heading("GENERATED REFERENT", |w| {
                w.field(
                    "REALIZATION",
                    &enum_render(req_str(generated, "realization")),
                );
                w.field(
                    "SPECIFICITY",
                    &enum_render(req_str(generated, "specificity")),
                );
            });
        }
        if let Some(denotation) = field_str(obj, "denotation") {
            w.field("DENOTATION", &enum_render(denotation));
        }
        if let Some(sd) = obj.get("scopeDependence").filter(|s| !s.is_null()) {
            render_scope_dependence(w, ctx, sd);
        }
        render_eventuality_dimensions(w, ctx, &key, obj);
        render_interval_modifiers(w, ctx, obj);
        render_arguments(w, ctx, obj);
        render_sign_content(w, ctx, obj);
        if let Some(descriptor) = obj.get("descriptor").filter(|d| !d.is_null()) {
            render_descriptor(w, ctx, descriptor);
        }
        render_assigned_names(w, ctx, obj);
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
        if let Some(params) = obj
            .get("parameters")
            .and_then(Value::as_array)
            .filter(|p| !p.is_empty())
        {
            // opt_terse_labels: PARAMETERS IN CALLABLE ORDER -> PARAMS.
            w.ordered("PARAMS", |w| {
                for p in params {
                    let param_key = pointer_key(p);
                    let param_sort = title_sort(req_str(ctx.object(param_key), "sort"));
                    w.entry(&format!("{} AS {param_sort}", ctx.id(param_key)));
                }
            });
        }
        if let Some(arity) = obj.get("arity") {
            w.field("ARITY", &number_str(arity));
        }
        if let Some(body) = obj.get("body") {
            let body_key = pointer_key(body);
            let body_type = req_str(ctx.object(body_key), "type");
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
        w.field("FORM", &enum_render(req_str(obj, "form")));
        // `value` is mandatory (the QuantityValue invariant requires exactly one
        // of integer/text/mathExpression); fail loudly, not tolerantly (round-2
        // review, Codex 6).
        let val = req_val(obj, "value").as_object().unwrap_or_else(|| {
            panic!("quantity `value` must be a JSON object (contract violated)")
        });
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
        w.field("SORT", &title_sort(req_str(obj, "sort")));
        w.field("ROLE", &enum_render(req_str(obj, "role")));
        w.field("INTRODUCED BY", &lexical(req_str(obj, "introducedBy")));
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
        if let Some(cc) = obj
            .get("connectionClaims")
            .and_then(Value::as_array)
            .filter(|c| !c.is_empty())
        {
            w.collection("CONNECTION CLAIMS", |w| {
                for c in cc {
                    w.entry(&ctx.id_of(c));
                }
            });
        }
        if let Some(be) = obj
            .get("boundEventualities")
            .and_then(Value::as_array)
            .filter(|b| !b.is_empty())
        {
            w.collection("BOUND EVENTUALITIES", |w| {
                for e in be {
                    w.entry(&ctx.id_of(e));
                }
            });
        }
        w.field("RELATION", &enum_render(req_str(obj, "relation")));
        if let Some(nc) = obj.get("nonlogicalConnection").filter(|n| !n.is_null()) {
            w.heading("NONLOGICAL CONNECTION", |w| {
                let operator = req_str(nc, "operator");
                if let Some(rest) = operator.strip_prefix("nonlogical:") {
                    w.field(
                        "OPERATOR",
                        &format!("{} {}", enum_render("nonlogical"), lexical(rest)),
                    );
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
        w.field("RELATION", &lexical(req_str(obj, "relation")));
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
    // Match the oracle: literal → its kind; else the operator; else no variant
    // at all (an unrecognized shape has no `IS <variant>` clause, not `IS ` with
    // an empty word).
    let variant: Option<String> = match literal {
        Some(literal) => Some(enum_render(req_str(literal, "kind"))),
        None => operator.map(enum_render),
    };
    w.declaration("MATH EXPRESSION", &vid, variant.as_deref(), true, |w| {
        if let Some(literal) = literal {
            let kind = req_str(literal, "kind");
            if kind == "integer" && literal.get("value").is_some() {
                w.field("VALUE", &number_str(req_val(literal, "value")));
            } else {
                w.field("NOT COMPUTED", &format!("math-literal-shape({kind})"));
            }
        } else if operator.is_some() {
            let empty = Vec::new();
            let operands = obj
                .get("operands")
                .and_then(Value::as_array)
                .unwrap_or(&empty);
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

/// A first-class semantic question. Required scalar fields fail loudly on
/// schema drift; empty optional slots are omitted, matching every other
/// optional collection in the notation.
#[requires(true)]
#[ensures(true)]
fn render_question(w: &mut Writer, ctx: &Ctx, key: &str, obj: &Value) {
    let vid = ctx.id(key).to_string();
    w.declaration("QUESTION", &vid, None, true, |w| {
        w.field("BODY", &ctx.id_of(req_val(obj, "body")));
        w.field("KIND", &enum_render(req_str(obj, "kind")));
        w.field("MODE", &enum_render(req_str(obj, "mode")));
        w.field("ASKER", &ctx.id_of(req_val(obj, "asker")));
        w.field("RESPONDENT", &ctx.id_of(req_val(obj, "respondent")));
        w.field("DOMAIN", &title_sort(req_str(obj, "domain")));
        if let Some(slots) = obj
            .get("slots")
            .and_then(Value::as_array)
            .filter(|slots| !slots.is_empty())
        {
            render_question_slots(w, ctx, slots);
        }
        if let Some(focus) = obj.get("focus") {
            w.field("FOCUS", &ctx.id_of(focus));
        }
        if let Some(answer) = obj.get("presupposedAnswer") {
            w.field("PRESUPPOSED ANSWER", &ctx.id_of(answer));
        }
        render_source(w, ctx, obj);
    });
}

/// Ordered answer slots for a question. Homogeneous slots carry only
/// `parameter`/`role`; typed slots additionally carry `kind`/`domain`, and a
/// typed truth slot legitimately omits `parameter`.
#[requires(!slots.is_empty())]
#[ensures(true)]
fn render_question_slots(w: &mut Writer, ctx: &Ctx, slots: &[Value]) {
    w.ordered("SLOTS", |w| {
        for (index, slot) in slots.iter().enumerate() {
            w.heading(&format!("[{}]:", index + 1), |w| {
                let kind = field_str(slot, "kind");
                let domain = field_str(slot, "domain");
                match (kind, domain) {
                    (None, None) => {
                        w.field("PARAMETER", &ctx.id_of(req_val(slot, "parameter")));
                    }
                    (Some(_), Some(_)) => {
                        if let Some(parameter) = slot.get("parameter") {
                            w.field("PARAMETER", &ctx.id_of(parameter));
                        }
                    }
                    _ => {
                        panic!(
                            "question slot must carry both `kind` and `domain`, or neither \
                             (contract violated)"
                        );
                    }
                }
                w.field("ROLE", &enum_render(req_str(slot, "role")));
                if let (Some(kind), Some(domain)) = (kind, domain) {
                    w.field("KIND", &enum_render(kind));
                    w.field("DOMAIN", &title_sort(domain));
                }
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn modal_arguments_share_host_args_and_hide_surface_introducers() {
        let objects = BTreeMap::from([
            ("entity:1".to_owned(), json!({ "type": "referent" })),
            ("entity:2".to_owned(), json!({ "type": "referent" })),
            ("formula:3".to_owned(), json!({ "type": "formula" })),
        ]);
        let id_map = BTreeMap::from([
            ("entity:1".to_owned(), "r1".to_owned()),
            ("entity:2".to_owned(), "r2".to_owned()),
            ("formula:3".to_owned(), "f3".to_owned()),
        ]);
        let host = json!({
            "arguments": {
                "x1": { "kind": "filled", "value": "entity:1" }
            },
            "modalArguments": [
                {
                    "relation": "pilno",
                    "introducedBy": "se pi'o",
                    "arguments": {
                        "x1": { "kind": "elided", "value": "entity:2" },
                        "x2": { "kind": "filled", "value": "entity:1" },
                        "x3": { "kind": "filled", "value": "entity:2" }
                    },
                    "source": {
                        "span": { "byteStart": 4, "byteEnd": 15 },
                        "text": "sepi'o lo ko'a",
                        "construct": "modal-argument"
                    }
                },
                {
                    "introducedBy": "fi'o broda",
                    "body": "formula:3",
                    "component": "entity:2"
                }
            ]
        });

        let ordinary_ctx = Ctx {
            objects: &objects,
            id_map: &id_map,
            provenance: false,
        };
        let mut writer = Writer::new(false, false);
        render_arguments(&mut writer, &ordinary_ctx, &host);
        let ordinary = writer.finish();
        assert_eq!(ordinary.matches("ARGS (").count(), 1, "{ordinary}");
        assert!(ordinary.contains("[1]: REFERENCE DENOTATION r1;"));
        assert!(ordinary.contains("[pilno]: ("));
        assert!(ordinary.contains("[3]: REFERENCE DENOTATION r2;"));
        assert!(ordinary.contains("[f3]: REFERENCE DENOTATION r2;"));
        assert!(!ordinary.contains("MODAL ARGUMENTS"));
        assert!(!ordinary.contains("se pi'o"));
        assert!(!ordinary.contains("fi'o broda"));
        assert!(
            ordinary.find("[1]:").expect("host numbered argument")
                < ordinary.find("[pilno]:").expect("modal predicate key"),
            "modal entries must follow numbered entries:\n{ordinary}"
        );
        assert!(
            ordinary.find("[pilno]:").expect("first modal")
                < ordinary.find("[f3]:").expect("second modal"),
            "modal JSON document order changed:\n{ordinary}"
        );

        let provenance_ctx = Ctx {
            objects: &objects,
            id_map: &id_map,
            provenance: true,
        };
        let mut writer = Writer::new(false, false);
        render_arguments(&mut writer, &provenance_ctx, &host);
        let provenance = writer.finish();
        for expected in [
            "INTRODUCED BY: se pi'o;",
            "INTRODUCED BY: fi'o broda;",
            "BYTE SPAN: 4..15;",
            "CONSTRUCT: \"tagged-argument\";",
        ] {
            assert!(
                provenance.contains(expected),
                "missing `{expected}`:\n{provenance}"
            );
        }
        assert_no_standalone_modal_word(&ordinary);
        assert_no_standalone_modal_word(&provenance);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn eventuality_without_arguments_gets_args_only_for_modals() {
        let objects = BTreeMap::from([("eventuality:1".to_owned(), json!({ "type": "referent" }))]);
        let id_map = BTreeMap::from([("eventuality:1".to_owned(), "r1".to_owned())]);
        let ctx = Ctx {
            objects: &objects,
            id_map: &id_map,
            provenance: false,
        };
        let host = json!({
            "modalArguments": [{
                "relation": "vanbi",
                "introducedBy": "va'o",
                "arguments": {
                    "x1": { "kind": "filled", "value": "eventuality:1" }
                }
            }]
        });

        let mut writer = Writer::new(false, false);
        render_arguments(&mut writer, &ctx, &host);
        assert_eq!(
            writer.finish(),
            "ARGS (\n  [vanbi]: (\n    [1]: REFERENCE DENOTATION r1;\n  )\n)\n"
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_no_standalone_modal_word(rendered: &str) {
        assert!(
            !rendered
                .split(|character: char| !character.is_ascii_alphanumeric())
                .any(|word| word.eq_ignore_ascii_case("modal")),
            "human-facing smusni must not use the standalone word `modal`:\n{rendered}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn historical_modal_source_and_locus_vocabulary_is_neutralized() {
        for (construct, rendered) in [
            ("modal-argument", "tagged-argument"),
            ("modal-indicator", "tagged-indicator"),
            ("modal-fragment", "tagged-fragment"),
            ("tense-modal-fragment", "tense-tag-fragment"),
            ("modal-branch-formula", "tag-branch-formula"),
            ("modal-connection-formula", "tag-connection-formula"),
        ] {
            assert_eq!(rendered_source_construct(construct), rendered);
            assert_no_standalone_modal_word(rendered);
        }

        let connector = json!({
            "source": "ki'u",
            "locus": "modal"
        });
        let mut writer = Writer::new(false, false);
        render_connector(&mut writer, Some(&connector), true);
        let rendered = writer.finish();
        assert!(rendered.contains("LOCUS: TAG;"));
        assert_no_standalone_modal_word(&rendered);
    }
}
