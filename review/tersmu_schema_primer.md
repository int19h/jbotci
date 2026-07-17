# `tersmu` semantic graph and derived-rendering reference

This is the authoritative reference for the JSON that `jbotci tersmu --format json` emits.
It is generated from the Rust model in `crates/jbotci-semantics/src/model.rs` and
`crates/jbotci-semantics/src/model/semantic_object.rs` (the structural source of
truth) and the builder in `crates/jbotci-semantics/src/generated_builder/`
(which maps Lojban syntax onto these objects). Use it to read a tersmu semantic
graph and judge whether it correctly captures the meaning of a Lojban utterance.

The flat `lojban-semantics-json-1` id-graph is the canonical model and the only
interchange form. `jbotci tersmu --format tree` and
`jbotci tersmu --format tree+proj` are human-readable projections computed
solely from that typed graph; neither is a canonical human syntax, and neither
adds or repairs semantic information. The `tree+proj` spelling follows the v0
`base+feature` convention: it is the `tree` base format plus a projected-
commitments section, and future format features use the same `+suffix` pattern.
In argv the `+` is literal. REST and MCP requests carry the same literal as a
JSON string: `"format": "tree+proj"`.

All field names below are the **exact JSON keys** (the model serializes with serde
`rename_all = "camelCase"` unless noted, so Rust `byte_start` → JSON `byteStart`, etc.). All enum
values are given exactly as they serialize (almost all enums are `camelCase`; a few are
`kebab-case` and are flagged). Optional fields are **omitted** when empty/`None` (serde
`skip_serializing_if`); never assume a missing key means a different value than "absent."

---

## Derived human-readable formats

### Interpretation contract

The following rules apply wherever a detached human projection is read:

- Indentation and `>` mean structural descent through the graph. The tree spine
  is authoritative wherever commitment follows structural position. An entry
  under `projected:` takes widest commitment scope even though its triggering
  predication is also shown at its structural tree site.
- `mode=` is the exact graph `PredicationMode` vocabulary (`asserted`,
  `definitional`, `restrictive`, `incidental`, `displayed`, `inert`,
  `performative`), not a second commitment level.
- `denotes` states a referential-identity commitment. A constant's
  `binder-dependence=fixed` means no graph binder was available at its
  introduction site. `binder-dependence=underspecified` plus
  `may-depend-on=...` names every available binder; it says the constant may
  co-vary with any of them, never that such dependence is proven. Indexicals
  are rigid and render `binder-dependence=fixed` without acquiring the
  constant-only JSON `scopeDependence` field.
- An eventuality with JSON `denotation:generated-bound` is not a referential
  constant. Its `binds=exists` annotation is structural existential scope at
  exactly one formula or sequence owner, not a projected claim. A
  `denotation:referential` eventuality instead participates in ordinary
  `denotes` commitments. `{event=...}` and `{tanru-head-event=...}` mark uses;
  the utterance event is its locution.
- Event condition suffixes always cover, in order, `time`, `actuality`,
  `aspect`, `recurrence`, `space`, `spatial-aspect`,
  `spatial-recurrence`, and `details`. `FIELD=unspecified` is an explicit
  absence-of-information marker, never atemporality, nonactuality, or another
  negative assertion. Populated sparse `details` ends in
  `otherwise=unspecified` so omitted members remain explicit. The human
  formats do not use a legend as a substitute for these per-site markers.

### `--format tree`

The structural tree prints utterance and sequence nesting followed by formula
nodes. Indentation is the graph's scope structure: `not`, quantifiers,
connectives, `restriction`, and `body` remain separate nodes. Atomic leaves use
the same typed predication and referent templates as tree+proj. A restricted
universal is annotated `domain-import=projective` on its quantifier line, while
the commitment itself remains visible in tree+proj's `projected:` section rather
than as a fabricated formula child. Descriptor bodies, relation/abstraction
bodies, eventuality abstraction content, and relative clauses retain their
graph edge labels as nested formula branches. Eventuality content uses
`abstraction content:` and is structural non-claim intensional content; it is
never reduced to an id in the event condition suffix. Restrictive, incidental,
and non-veridical restrictive clauses have distinct branch labels; displayed
asides are printed at their utterance attachment. Question objects render an
explicit `question body:` branch, and parsed quotation signs render their
structured utterance below `quotation:`. Both regions are non-claim content.
Descriptor operands use `descriptor operand:`; this is the typed bridge that
keeps a parsed quotation reachable through forms such as `la'e lu ... li'u`.

The tree is deterministic in semantic child order: connective children keep
their stored order, restrictions precede quantifier bodies, sequence connection
claims precede items, and ties use object id. Repeated ids are intentionally
shown; equality of ids, not repeated label text, expresses sharing.
Referent expansion follows typed field order: the descriptor subgraph (body,
operand, then relative clauses), parsed quotation content for sign referents,
eventuality abstraction content, an optional intensional body, then
referent-level relative clauses. Formula and sequence binding owners descend
their `boundEventualities` after their ordinary children; this makes content on
an owner-only generated event structurally reachable without moving its
existential scope. The traversal's active object/formula set stops cycles
through content, body parameters, quotations, and self-references.

### `--format tree+proj`

The tree+proj projection is a partition, not a concatenation of two formats.
Its byte-stable shape is the structural tree spine, one blank line, and a
single `projected:` section:

```text
<structural tree spine>

projected:
- <only displaced commitment, or (none)>
```

The tree spine is authoritative wherever commitment follows structural
position. There is no at-issue ledger tier and no `context=` breadcrumb:
indentation and branch labels are the context. At-issue predications, displayed
content, and non-claim intensional relation/abstraction bodies and eventuality
content occur only in the tree. Displayed asides already contain their full
fixed payload; relation/abstraction branches retain their explicit
`relation body:` or `abstraction body:` role marker, and eventuality content
retains its explicit `abstraction content:` role marker.

The `projected:` section enumerates only commitments that escape their tree
site:

- restricted-universal domain imports use the same explicit witness,
  restriction id, and owner id;
- veridical descriptor bodies and incidental/restrictive clauses repeat their
  full predication text and exact graph `mode=`. The duplication with the tree
  is intentional: it records that the commitment escapes that position;
- ordinary referential constants use
  `denotes LABEL [binder-dependence=STATE; constant]` (with the full event
  condition suffix before binder dependence when that denotation is the
  referential event's unique condition site);
- indexicals and locutions compress to exactly one `frame:` line. Indexicals
  and locutions each retain an explicit `binder-dependence=fixed` qualifier;
  an eventuality indexical such as `now` carries its condition suffix there,
  while locution conditions stay on the utterance line;
- typed implicit-place constants with the same `DescriptorKind` and the same
  binder-dependence value share one line:
  `denotes [LABEL, ...] [binder-dependence=STATE; constant;
  descriptor-kind=KIND]`. Current group kinds are `elided` (`zo'e`) and
  `typical-place-value` (`zu'i`). Every label/id remains present, and constants
  with different candidate binders can never share a qualifier.

Event conditions occur once per event in this format. Generated events carry
the full suffix only at their `binds=exists` formula/sequence owner; their
predication/connective use sites retain `{event=...}` without another suffix.
Locutions carry it on their utterance; event indexicals carry it on `frame:`;
other referential events carry it on their `denotes` line. Projected copies of
predications and domain-import restriction text likewise retain the event id
but omit a repeated condition suffix. Thus the tree+proj format preserves the
explicit-absence doctrine without echoing conditions through use markers or
context fragments.

Tree+proj ordering is fixed. The tree retains normal semantic child order. The
projected section emits `frame:` first, displaced formula commitments in
semantic traversal order, ordinary referential constants in first-visit order,
then implicit groups in typed kind/binder order. A semantic identity is emitted
at most once in each applicable projected category.

The CLI, REST, and MCP request `format` values are `json`, `tree`, and
`tree+proj`. All three surfaces default to `tree+proj`; request `json`
explicitly whenever the canonical interchange graph is required.

---

## 1. Top-level shape

```json
{
  "version": "lojban-semantics-json-1",
  "root": "<object-id>",
  "objects": { "<object-id>": { ...object... }, ... }
}
```

- **`version`** — always the literal string `"lojban-semantics-json-1"`.
- **`root`** — the id of the entry-point object for this graph. It is typically an
  `utterance:5`, a `sequence:17` (multiple top-level utterances), or for a bare fragment a
  `formula:12`.
- **`objects`** — a map from **id string** to object. Every object referenced anywhere in the
  graph is defined here (the graph is validated to have no dangling references), and `root` is
  always a key in this map.

### Object id conventions

Ids are strings of the form `«prefix»:«global-index»`. The numeric suffix is
globally unique across the graph and assigned in build order; it has no semantic
meaning, but identity (the same complete id used twice) **does**. Referential
prefixes are the serialized sort, including slash-separated eventuality
subsorts. Structural prefixes are the object kind.

| Id examples | public `type` | Meaning |
|---|---|---|
| `entity:1`, `mass:20`, `relation:14`, `sign:6` | `referent` | non-event referent; prefix is `sort` |
| `eventuality:3`, `eventuality/state:20`, `eventuality/locution:15` | `referent` | eventuality referent; prefix is the complete sort path |
| `utterance:5`, `sequence:17`, `parameter:8`, `predication:12`, `formula:13` | corresponding structural type | structural nodes |
| `display:9`, `math:10`, `quantity:11`, `relationMetadata:12`, `question:13` | `displayedContent`, `mathExpression`, `quantity`, `relationMetadata`, `question` | remaining structural nodes |

The public `type` values are exactly `utterance`, `sequence`, `referent`,
`parameter`, `predication`, `formula`, `displayedContent`, `mathExpression`,
`quantity`, `relationMetadata`, and `question`. Eventualities, signs, and
abstraction outputs are specialized referent shapes rather than separate
public `type` values.

---

## 2. Object types and their fields

`SemanticObject` is an enum of kind-specific validated node structs. A custom
serializer exposes their common flat JSON boundary; it is not one permissive
Rust struct with unrelated optional fields. The `type` field is always present.
Two fields are universal:

- **`source`** — optional `SemanticSource` (see §4); provenance/span. Present on most objects.
- **`diagnostics`** — optional array of `{ "severity": ..., "message": ... }`; `severity` ∈
  `info` | `warning` | `error` (currently always `warning` in practice). Omitted when empty.

The full inventory of fields and their enum domains follows by object type.
Fields not listed for a type are not part of that node's public shape.

### 2.1 `utterance`

A complete speech act.

| Key | Type | Meaning |
|-----|------|---------|
| `force` | `UtteranceForce` | `assert`, `ask`, `command`, `mention`, `quote`, `parenthetical`, `subordinated`, or `vocative` |
| `speaker` | id → `referent` | the speaker; normally an `indexical:"speaker"` entity |
| `audience` | id → `referent` | the addressee; normally an `indexical:"audience"` entity |
| `eventuality` | id → `referent` | the `eventuality/locution` referent for this utterance |
| `content` | id | optional content. Ordinary utterances use a `formula`, `sequence`, or `question`; mention/quote/vocative content may be an argument-fillable object |
| `deicticGround` | `DeicticGround` | `{ "time": "eventuality:3", "place": "entity:4" }`; this utterance's `now`/`here` indexicals |
| `asides` | array of id → `utterance` or `displayedContent` | parenthetical/vocative/displayed side content attached here |
| `vocativeKind` | string | present for vocative utterances |

`deicticGround` is set on every utterance. Top-level sibling utterances share
the four frame indexicals; parsed quotations allocate a fresh frame.

### 2.2 `sequence`

An ordered grouping of top-level utterances/sequences (e.g. several `.i`-joined statements, or a
text with multiple paragraphs).

| Key | Type | Meaning |
|-----|------|---------|
| `items` | array of id | members, each an `utterance`, `sequence`, or `displayedContent` |
| `force` | `UtteranceForce` | optional; when present, exactly `subordinated` |
| `relation` | `SequenceRelation` | `same-topic-continuation`, or tagged `paragraph-boundary` with `transition` and ordered `additional` transitions (**kebab-case**) |
| `connectionClaims` | array of id → `formula` | formulas asserting the logical connection between consecutive items (for `.i je`-style sentence connectives) |
| `content` | id → `formula` or `question` | aggregate logical/question content, when present |
| `boundEventualities` | array of id → generated eventuality | typed existential-binding edge used when the event's formula roots have no formula LCA |
| `ordinalLabels` | array of `OrdinalLabel` | truth-inert `mai`/`mo'o` labels |
| `nonlogicalConnection` | `{ operator, connector }` | JOI-family statement connection metadata; never a truth formula |
| `elidedConnectionOperand` | `priorDiscourse` or `followingDiscourse` | missing outside operand of a leading/trailing statement connection |

`SequenceRelation::ParagraphBoundary` serializes as
`{"paragraph-boundary":{"transition":"new-topic","additional":[]}}`;
transition values are `new-topic` (`ni'o`) and `resume-prior-topic` (`no'i`).
As of this snapshot, the builder retains that relation on a standalone or
trailing NIhO boundary but incorrectly falls back to
`same-topic-continuation` when a nonempty following paragraph is present
(#447). This is an implementation bug, not the intended meaning of the enum.

### 2.3 eventuality `referent`

The event/state/process introduced by a bridi (and the locution event of an
utterance). Its public `type` is `referent`; `sort` distinguishes the broad
eventuality and its slash-separated subsorts. Tense, aspect, and spatial
information hang off this referent shape.

| Key | Type | Meaning |
|-----|------|---------|
| `denotation` | `EventualityDenotation` | required: `generated-bound` for a generated witness, or `referential` for a denoted event |
| `sort` | `SemanticSort` | `eventuality`, `eventuality/state`, `eventuality/process`, `eventuality/activity`, `eventuality/achievement`, `eventuality/experience`, or `eventuality/locution` |
| `category` | `ReferentCategory` | present only for `denotation = referential` |
| `scopeDependence` | `ScopeDependence` | present exactly for referential constants; generated-bound co-variation is structural |
| `indexical` | `IndexicalKind` | present for indexical `now` eventualities |
| `descriptor` / `composition` / `relativeClauses` / `assignedNames` | referent metadata | the same roles as on non-event referents |
| `modalArguments` | array of `ModalArgument` | event-level modal arguments |
| `actuality` | `Actuality` = `{ "kind": ActualityKind }` | `kind` ∈ `actual`, `capable`, `potential`, `demonstrated` (from CAhA: ca'a/ka'e/nu'o/pu'i) |
| `content` | id | the formula or sequence this eventuality is the occurrence of |
| `body`, `parameters`, `arity`, `embeddedQuestions` | abstraction fields | direct event abstraction output; see §3.6 |
| `experiencer`, `scale`, `target`, `subscript` | optional ids/metadata | abstraction, scalar, shift, and subscript details |
| `tenseModal` | id → `parameter` | present when the tense itself is questioned (`cu'e`); the parameter has sort `tenseModal`, role `tenseQuestion` |
| `time` | `AnchorRelation` | primary temporal placement (see §3.7) |
| `timePath` | array of `TemporalPathStep` | chained temporal offsets |
| `timeInterval` | `TimeInterval` = `{ extent, anchor? }` | ZEhA-style duration |
| `timeSpan` | `TimeSpan` = `{ start, end, introducedBy }` | bounded span (each endpoint `{ relation, anchor?, introducedBy, distance?, scalarNegation? }`) |
| `aspect` / `aspects` | `Aspect` = `{ contour, anchor?, scalarNegation? }` | ZAhO event contour(s) |
| `recurrence` | array of `Recurrence` | ROI/TAhE/etc. repetition; see §2.x note below |
| `intervalModifiers` | array of tagged `Aspect`/`Recurrence` | canonical temporal interval-property stack |
| `space` | `AnchorRelation` | primary spatial placement |
| `spacePath` | array of `TemporalPathStep` | chained spatial offsets |
| `spaceInterval` | `SpaceInterval` = `{ extent?, directions[], dimensions[], anchor? }` | VEhA/VIhA |
| `spatialAspect` / `spatialAspects` | `Aspect` | spatial contour |
| `spatialRecurrence` | array of `Recurrence` | |
| `spatialIntervalModifiers` | array of tagged `Aspect`/`Recurrence` | canonical spatial interval-property stack |

Every `generated-bound` event occurs in exactly one owner's
`boundEventualities`. The owner is the lowest formula dominating every primary,
ordinary/modal-argument, transitive-tanru, formula-event, and `content` use; if
no formula LCA exists, the lowest containing sequence owns it. Referential
events—including `lo`/`le nu` denotations and promoted body events, locutions,
indexicals, and mentioned fragments—never occur on the edge. Generated events
omit `category` and `scopeDependence` entirely.

`Recurrence` fields: `kind` ∈ `occurrenceCount`, `ordinalOccurrence`, `regular`, `typically`,
`continuously`, `habitually`; plus `introducedBy`, optional `connection`
(`{kind:"product", introducedBy}`), `quantity` (id → first-class `quantity`) or legacy/direct
`value` (`QuantityValue`), `interval` (id), and `negation`
(`ModalNegation` `{kind:"contradictory", introducedBy}`).

### 2.4 `referent`

A thing that can fill an argument place — the semantic value of a sumti.

| Key | Type | Meaning |
|-----|------|---------|
| `category` | `ReferentCategory` | `constant`, `variable`, `indexical`, `composite` |
| `scopeDependence` | `ScopeDependence` | present exactly for constants: `{ "kind": "fixed" }` or `{ "kind": "underspecified", "mayDependOn": [id, ...] }` |
| `sort` | `SemanticSort` | semantic sort (see full list below) |
| `indexical` | `IndexicalKind` | present when `category = indexical`: `speaker`, `audience`, `now`, `here`, `proximalDemonstrative`, `medialDemonstrative`, `distalDemonstrative` |
| `descriptor` | `Descriptor` | how this referent was described (le/lo/la/pro-sumti/etc.); see §3.1 |
| `composition` | `Composition` | for `category = composite` (set/mass/connected/interval sumti); see below |
| `relativeClauses` | array of `RelativeClause` | poi/noi attached to the referent (see §3.8) |
| `assignedNames` | array of `AssignedName` | goi/cei name assignments: `{ name, word, introducedBy, source? }` |
| `body`, `parameters`, `arity`, `embeddedQuestions` | abstraction fields | direct non-event abstraction output; see §3.6 |
| `experiencer`, `scale`, `target`, `subscript` | optional ids/metadata | abstraction, scalar, shift, and subscript details |

**`ReferentCategory`** values: `constant` (a constant denotation, e.g. names,
`lo`-descriptions, plain pro-sumti, and elided `zo'e`), `variable` (bound logical
variable da/de/di), `indexical` (mi/do/ti/ta/tu and deictic referents), `composite`
(built from members via `composition`). A constant is not necessarily fixed across
the values of an enclosing binder.

**`ScopeDependence`** is explicit on every constant and absent on every non-constant.
`fixed` means no binder is in scope at the constant's introduction site.
`underspecified` has a nonempty, sorted `mayDependOn` set containing exactly the
typed formula, abstraction, or question binders in scope there. It means the
constant **may** co-vary with any named binder; it never claims that dependence
actually occurs. The builder derives this field from the rooted typed graph. At
quantifier formulas the variable scopes over the restriction and body; coequal
bundle/respective binders scope over their restrictions and body; abstraction
parameters scope over their body; question slots scope over their question body;
nested utterances reset the binder environment. The introduction site is the first
reference to the constant in canonical semantic traversal order (including
restriction-before-body and sequence-connection-claims-before-items order). Shared
later references preserve that one derived value. Disconnected objects, which normal
generated graphs prune, are treated as ID-ordered roots at empty scope so validation
is total.

**`SemanticSort`** (the `sort` field, also used for `domain`): `entity`, `mass`, `set`,
`sequence`, `time`, `eventuality`, `eventuality/state`, `eventuality/process`,
`eventuality/activity`, `eventuality/achievement`, `eventuality/experience`,
`eventuality/locution`, `predication`, `truthValue`, `proposition`, `concept`, `amount`,
`quantity`, `number`, `scale`, `text`, `sign`, `relation`, `place`, `connective`, `tenseModal`,
`mathOperator`, `argumentBundle`, `abstractNature`.

**`Descriptor`** struct keys: `kind` (string — see the enumerated values in §3.1), `word` (the
source cmavo, e.g. `"le"`, `"lo"`, `"la"`, `"ti"`), optional `speaker` (id → referent, used by
speaker-relative descriptions), optional `body` (id → formula: the selbri restriction, e.g. for
`lo broda` the formula `broda(this)`), optional `veridical`, `relativeClauses`,
optional `quantity` (id → quantity), optional `name` (string, for `la`-names),
optional `scale` (id → referent), optional `definiteness` (`affirmedPoint`,
`indefiniteAlternative`, `neutralPoint`, or `uniqueExtreme`), and optional
`operand` (id — the inner object a
descriptor wraps/reuses; **not** only `li`/math operands: also the raised sumti of a
`tu'a`/`jai` raising (`descriptor.kind: abstractionAbout`), the `la'e`/`lu'e`
reference↔referent shift target, and the base referent of a `NAhE`/qualifier
(`otherThan`/`oppositeOf`/`neutralOf`)).

**`Composition`** struct keys: `operator` (`connectiveQuestion`, `joint`,
`mass`, `set`, `sequence`, `respectively`, `union`, `intersection`,
`crossProduct`, `unorderedInterval`, `orderedInterval`, or `centeredInterval`), optional
`operatorParameter` (id → parameter, only and always present iff `operator == "connectiveQuestion"`),
`members` (array of argument-fillable ids), `excludedMembers` (array), optional `collective`
(bool), optional `scalarNegated` (bool), optional `complement` (bool, interval complement),
optional `endpointInclusion` (for interval operators).

### 2.5 `parameter`

A placeholder/variable that is bound or questioned elsewhere (question words `ma`/`mo`, property
slots `ce'u`, relative heads `ke'a`, connective questions `je'i`, tense questions `cu'e`, etc.).

| Key | Type | Meaning |
|-----|------|---------|
| `sort` | `SemanticSort` | the sort of value that fills it |
| `role` | `ParameterRole` | `propertySlot`, `relativeClauseHead`, `argumentQuestion`, `relationQuestion`, `relationVariable`, `unspecifiedRelation`, `placeQuestion`, `connectiveQuestion`, `tenseQuestion`, `mathOperatorQuestion`, `quantityQuestion`, `attitudeQuestion`, `respectiveSlot` |
| `introducedBy` | string | the source cmavo (e.g. `"ma"`, `"ce'u"`, `"ke'a"`, `"cu'e"`) |

The validator enforces sort/role coherence: e.g. `relationQuestion`/`relationVariable` ⇒ sort
`relation`; `placeQuestion` ⇒ sort `place`; `connectiveQuestion` ⇒ sort `connective`;
`tenseQuestion` ⇒ sort `tenseModal`; `mathOperatorQuestion` ⇒ sort `mathOperator`;
`unspecifiedRelation` ⇒ sort `relation`; `quantityQuestion` ⇒ sort `number`;
`argumentQuestion`/`relativeClauseHead`/`attitudeQuestion` ⇒ sort `entity`;
`propertySlot` and `respectiveSlot` may use any semantic sort.

### 2.6 `predication`

An applied relation — a selbri applied to its arguments. This is the atomic content under a
`formula` of operator `atom`.

| Key | Type | Meaning |
|-----|------|---------|
| `relation` | string | the predicate name (gismu/lujvo/brivla root, e.g. `"klama"`); present iff `relationParameter` is absent |
| `relationParameter` | id → parameter | present instead of `relation` when the relation itself is questioned/variable (`mo`, relation variable) |
| `eventuality` | id → eventuality | the eventuality this predication occurs in (carries the tense/aspect) |
| `arguments` | map `"x«n»"` → `ArgumentValue` | the filled/elided/deleted argument places, keyed by numbered place (see §3.5 for SE place remapping) |
| `mode` | `PredicationMode` | `asserted`, `definitional`, `restrictive`, `incidental`, `displayed`, `inert`, `performative` |
| `placeQuestions` | array of `PlaceQuestionBinding` | for questioned places (`fi'a`-style): `{ parameter, argument, candidatePlaces[], source? }` |
| `modalArguments` | array of `ModalArgument` | BAI/`fi'o` modal places attached to this predication (see §3.7) |
| `reciprocity` | array of `ReciprocalExchange` | `soi`-style reciprocal place exchanges `{ left, right, introducedBy, source? }` |
| `scalarNegation` | `ScalarNegation` | scalar negation of the predicate itself (na'e/to'e/no'e/je'a on the selbri) |
| `relationMetadata` | id → relationMetadata | optional link to a `relationMetadata` object describing the relation's place structure |

**`ArgumentValue`** keys: `kind` ∈ `filled` (overt sumti), `elided` (default/implicit value
supplied, with `introducedBy` recording the elision marker e.g. `"zo'e"`), `deleted` (place
suppressed, `zi'o`; carries `introducedBy`, no `value`); optional `value` (the filling id;
required for `filled`/`elided`, absent for `deleted`), optional `quantity` (id → quantity, inner
quantifier), optional `introducedBy`, optional `source`, optional `relativeClauses`.

**`ModalArgument`** keys: `relation` (string, e.g. `"ki'u"`/`"mu'i"`/a `fi'o`-selbri name),
`introducedBy`, `arguments` (map `"x«n»"` → ArgumentValue), optional `negation` (ModalNegation),
optional `scalarNegation`, optional `modifiers`, optional `source`. The `relation` is the
**unconverted** root selbri (e.g. `fi'o se pilno` → `relation: "pilno"`) and `arguments`
uses the **SE-remapped** place numbering, exactly as `predication.arguments` does (§3.5) —
so `fi'o se pilno`'s tagged sumti correctly lands in the base x2 (tool) place.

### 2.7 `formula`

A truth-valued logical formula. Its **`operator`** field selects the shape:

| `operator` | Shape (which fields are populated) | Meaning |
|-----------|-------------------------------------|---------|
| `atom`     | `predication` (id → predication)    | an atomic claim |
| `affirmed` | `children` (1 element)              | explicit bridi-level `ja'a` affirmation |
| `not`      | `children` (1 element)              | logical negation (na, selbri negation, connective `nai`) |
| `scoped`   | `children`                          | an explicit scope boundary |
| `and`      | `children`, optional `connector`    | conjunction |
| `or`       | `children`, optional `connector`    | disjunction |
| `implies`  | `children`, optional `connector`    | material implication (`na.a`/`na ja` etc.) |
| `iff`      | `children`, optional `connector`    | biconditional (o/jo/go/gi'o) |
| `exclusiveOr` | `children`, optional `connector` | exclusive or |
| `whetherOrNot` | `children`, optional `connector` | u/ju/gu/gi'u "whether-or-not" |
| `connectiveQuestion` | `children`, `connector` (with `parameter`) | a questioned connective (`je'i`) |
| `exists`   | `variable`, optional `restriction`, `body`, optional `quantity` | existential quantification |
| `forall`   | (same as exists), plus `domainImport` iff restricted | universal quantification |
| `none`     | (same as exists)                    | "zero / no x" quantification (`no`) |
| `cardinality` | (same as exists) + `quantity`    | numeric quantification (`ci`, `re`, ...) |
| `pluralExists` | (same as exists)                 | plural existential |
| `pluralForall` | (same as exists), plus `domainImport` iff restricted | plural universal |
| `quantifierBundle` | `bindings`, `body`, `coequalScope:true` | coequal grouping-termset binders |
| `respectivelyDistribution` | `streams`, `body`, optional `distinctPartition` | truth-conditional `fa'u` zip |

Every formula shape may carry **`boundEventualities`**, a nonempty array of
generated-event IDs existentially bound at that exact formula. Each generated
event appears on exactly one formula or sequence owner, and referential events
are forbidden from the array.

Quantified-formula fields:
- **`variable`** — id → referent or parameter being bound.
- **`restriction`** — optional id → formula limiting the variable's domain.
- **`domainImport`** — present with the sole value `projective` exactly when
  `operator` is `forall` or `pluralForall` and `restriction` is present. It
  says the restriction domain is nonempty and that this commitment projects
  past `not`; it is not an at-issue conjunct.
- **`body`** — id → formula, the scope.
- **`quantity`** — optional id → quantity giving the count/form.

Connective-formula field:
- **`children`** — array of id → formula (the connected operands; for `and`/`or` etc. these are
  the two sides; nested connectives nest as child formulas).
- **`connector`** — optional `Connector` = `{ source, locus, truthTable?, parameter? }`:
  - `source` — e.g. `"logical-connective"` / `"nonlogical-connective"`.
  - `locus` — where the connective sat grammatically: e.g. `"sumti"`, `"selbri"`, `"bridi"`,
    `"tanru"`, `"tense"`.
  - `truthTable` — the full connective surface text (records the exact word incl. se/na/nai).
  - `parameter` — id → parameter, present only for a questioned connective
    (`operator = connectiveQuestion`); the parameter has sort `connective`, role
    `connectiveQuestion`.

### 2.8 abstraction fields on referents

NU abstractions have no separate `abstraction` object. The abstraction output
is an eventuality or non-event referent whose own fields carry the reified
content.

| Key | Type | Meaning |
|-----|------|---------|
| `content` or `body` | id → formula/sequence | event abstractions use `content`; proposition/relation/etc. outputs use `body` |
| `parameters` | array of id → parameter | the abstracted slots (e.g. `ce'u` slots for `ka`) |
| `arity` | int | set for `property` (= number of parameters) |
| `embeddedQuestions`, `experiencer`, `scale`, `target` | optional details | indirect questions and abstraction-specific payload |

Public JSON intentionally omits the internal `abstractionKind` and eventuality
`class` discriminators. See §3.6 for the cmavo-to-sort/content mapping that
identifies the output shape on the wire.

### 2.9 sign `referent`

A `type:"referent"`, `sort:"sign"` metalinguistic value: a quotation,
letteral string, math expression as text, connective word, single word, or text.

| Key | Type | Meaning |
|-----|------|---------|
| `category` | `ReferentCategory` | sign referents may be constants or another non-indexical category |
| `scopeDependence` | `ScopeDependence` | present exactly when `category = constant`; see §2.4 |
| `sort` | `SemanticSort` | always `sign` |
| `descriptor` | `Descriptor` | optional descriptor metadata |
| `kind` | `SignKind` | serialized from `sign_kind`: `quotation`, `letteral`, `mathExpression`, `connective`, `word`, `text` |
| `text` | string | literal text for non-quotation signs |
| `letterals` | array of `LetteralUnit` | for letteral strings (BY etc.); see below |
| `quotation` | `Quotation` | for quotations: `{ mode, utterance?, delimiter?, text? }`. **`mode` is the structural category, not the delimiter cmavo**: `parsed` (structured `lu…li'u` — an `utterance` id is reachable from outside) or `opaque` (sealed `zo`/`lo'u`/`zoi` — no reachable referents). The surface delimiter is the separate `delimiter` field; `text` holds the raw source. |
| `denotes` | id | what the sign denotes (referent or mathExpression) |
| `relativeClauses`, `target`, `subscript` | optional metadata | clauses, shift target, and XI subscript |

**`LetteralUnit`** keys: `kind` ∈ `glyph`, `digit`, `shift`, `characterCode`, `compound`;
`sourceWords[]`; optional `text`, `value`, `modifier`, `buDepth` (only when > 0); `parts[]` (only
for `compound`).

### 2.10 `displayedContent`

Indicator/attitudinal content (UI/CAI emotion words, evidentials, discursives, metalinguistic
markers) — content that is *displayed* rather than asserted.

| Key | Type | Meaning |
|-----|------|---------|
| `family` | `DisplayedContentFamily` | `emotion`, `attitudeModifier`, `propositionalAttitude`, `evidential`, `discursive`, `metalinguistic`, `emphasis`, `questionPrompt` |
| `relation` | string | the indicator's predicate name |
| `polarity` | `DisplayedContentPolarity` | `positive`, `neutral`, `negative` (from nai/cu'i) |
| `assertionEffect` | `DisplayedContentAssertionEffect` | `none`, `hostAsserted`, `hostSubordinated`, `metalinguisticallyVoided`, `performative` |
| `intensity` | string | intensity word (CAI: cai/sai/ru'e/...) |
| `phase` | string | |
| `experiencer` | id → referent | who holds the attitude (usually speaker) |
| `target` | id | what it applies to (an utterance, or argument-fillable object) |
| `targetFocus` | `bridi` or `selbri` | optional metalinguistic/surface focus |
| `anchor` | id → utterance | the host utterance |
| `modifiers` | array of `DisplayedContentModifier` | nested attitude modifiers `{ relation, family?, polarity?, intensity?, assertionEffect?, source? }` |

### 2.11 `mathExpression`

A MEX/`li` mathematical expression.

| Key | Type | Meaning |
|-----|------|---------|
| `operator` | string | the math operator name (present for compound expressions; for intervals ends with `"Interval"`) |
| `operatorParameter` | id → parameter | when the operator is questioned (sort `mathOperator`, role `mathOperatorQuestion`); mutually exclusive with `operator`/`literal` |
| `operands` | array of id → mathExpression | sub-expressions |
| `literal` | `MathLiteral` = `{ kind, value }` | leaf value; `value` is an integer or a string (untagged). E.g. `mo'e`→`{kind:"sumtiOperand", value:"mo'e"}`, `ni'e`→`{kind:"selbriOperand", value:"ni'e"}`, plain numbers → `{kind:"integer", value: N}` |
| `denotes` | id | for literal `mo'e`/`ni'e` operands: the referent or selbri-derived object the operand denotes |
| `operatorDenotes` | id | denotation attached to an operator form |
| `endpointInclusion` | `IntervalEndpointInclusion` = `{ left, right }` | each ∈ `inclusive`/`exclusive`; only for `"...Interval"` operators |
| `scalarNegation`, `subscript` | optional metadata | scalar NAhE and XI structure |

### 2.12 `quantity`

A quantifier/number value with a form and scale.

| Key | Type | Meaning |
|-----|------|---------|
| `form` | `QuantityForm` | `exact`, `all`, `atLeast`, `atMost`, `moreThan`, `lessThan`, `approximate`, `indefinite`, `enough`, `tooMany`, `tooFew` |
| `value` | `QuantityValue` | exactly one of `integer` (int), `text` (string), or `mathExpression` (id) is present; `questionParameters` may additionally list number-sorted `quantityQuestion` parameters |
| `scale` | `QuantityScale` | `count`, `fraction`, `ordinal`, `amount`, `extent`, `frequency` |
| `comparisonSet` | id | for relative quantifiers |

### 2.13 `relationMetadata`

Descriptive metadata about a relation's place structure (not part of the logical content; an
annotation).

| Key | Type | Meaning |
|-----|------|---------|
| `relation` | string | relation name |
| `sourceWords` | array of string | source word(s) |
| `placeStructure` | array of `PlaceDescription` = `{ place, description }` | per-place glosses |
| `expansion` | `RelationExpansion` = `{ kind, sourceWords[], rafsiBindings[] }` | lujvo/expansion info; each `RafsiBinding` = `{ rafsi, sourceWord?, referent? }` |

Current emission is narrower than the type permits: the builder creates this
object for a lujvo only when it finds a context-sensitive pro-sumti rafsi
binding. Ordinary lujvo such as `dalmikce`, `ctigau`, and `gerzda` currently
have no `relationMetadata` link or object (#450). Consequently, `ctigau` also
does not retain the implicit `nu citka` event content required for an
implicit-abstraction lujvo (#451).

### 2.14 `question`

An explicit question abstraction (truth questions, fill-in questions, etc.).

| Key | Type | Meaning |
|-----|------|---------|
| `kind` | `QuestionKind` | `truth`, `argument`, `relation`, `place`, `connective`, `tense`, `mathOperator`, `attitude`, `quantity`, or `multiple` |
| `mode` | `QuestionMode` | serialized from `question_mode`: `direct`, `indirect` |
| `domain` | `SemanticSort` | the sort being asked about |
| `body` | id → formula | the question's open formula |
| `slots` | array of `QuestionSlot` | homogeneous `{ parameter, role }` slots, or typed `{ parameter?, role, kind, domain }` slots for `kind:multiple` |
| `asker` | id → referent | usually the speaker |
| `respondent` | id → referent | usually the addressee |
| `focus` | id → parameter \| referent | optional focused element |
| `presupposedAnswer` | id → parameter \| referent | optional presupposition |

Homogeneous slots inherit their question's `kind` and `domain`. Every slot in
a `multiple` question is typed; a truth slot omits `parameter`. Slot roles are
`answer` and `respectiveSlot`. Embedded indirect questions are referenced by
the containing abstraction-output referent's `embeddedQuestions`, not by a
question object.

---

## 3. How Lojban constructs are expected to appear

### 3.1 Descriptions: `le` vs `lo` vs `la`

A description sumti produces a `referent` whose `descriptor.kind` records the article and whose
`descriptor.body` (when a selbri is present) is the restriction formula `selbri(thisReferent)`.
The descriptor `word` holds the source article cmavo. The exact `descriptor.kind` values:

| Article | `descriptor.kind` | default `sort` |
|---------|-------------------|----------------|
| `lo`    | `veridicalDescription` | `entity` |
| `loi`   | `veridicalMassDescription` | `mass` |
| `lo'i`  | `veridicalSetDescription` | `set` |
| `le`    | `speakerDescription` | `entity` |
| `lei`   | `speakerMassDescription` | `mass` |
| `le'i`  | `speakerSetDescription` | `set` |
| `le'e`  | `speakerStereotypeDescription` | `entity` |
| `lo'e`  | `typicalDescription` | `entity` |
| `la`    | `name` | `entity` |
| `lai` + selbri  | `massNameDescription` | `mass` |
| `la'i` + selbri | `setNameDescription` | `set` |
| `lai` + cmevla  | `massName` | `mass` |
| `la'i` + cmevla | `setName` | `set` |
| (other / fallback) | `description` | `entity` |

**`lai`/`la'i` split is intentional** (spec amendment 29): a `lai`/`la'i` over a **selbri**
is a *description* → `massNameDescription`/`setNameDescription`; over **cmevla** name-words it
is a bare *name* → `massName`/`setName`. (Plain `la` is `name` on both paths.) Do not flag this
as an inconsistency.

The table above lists the article-bearing kinds. The remaining exact enum
values are `number`, `scale`, `proSumti`, `unloweredSumti`,
`typicalPlaceValue`, `utteranceReference`, `elided`, `abstractionAbout`,
`referentOfSymbol`, `symbolForReferent`, `memberOf`, `setFrom`, `massFrom`,
`sequenceFrom`, `qualifiedSumti`, `oppositeOf`, `neutralOf`, `affirmedAs`, and
`otherThan`. These are valid descriptor shapes, not fallbacks; an unrecognized
value is schema drift rather than an open extension point.

Reviewer expectations:
- **`lo broda`** — `category: constant`, `descriptor.kind: veridicalDescription`, `descriptor.word: "lo"`,
  `descriptor.body` → a formula `broda(r)` (the restriction). The referent is the veridical thing(s)
  that *are* broda.
- **`le broda`** — `descriptor.kind: speakerDescription`; the speaker's intended referent(s),
  described as broda but not asserted to be.
- **`la broda` / `la .cmevla.`** — `descriptor.kind: name`; for cmevla names the `descriptor.name`
  string and/or `assignedNames` carry the name; the body need not be a veridical claim.
- When the selbri is itself a NU-abstraction (e.g. `lo nu broda`), the direct
  output referent's top-level `content` or `body` carries the abstraction; the
  descriptor need not repeat that body. Its `sort` follows the abstraction's
  output sort (eventuality/proposition/etc.).

### 3.2 Pro-sumti: `mi`, `do`, `ti`, `ta`, `tu`, da/de/di, zo'e, etc.

| Word | Result |
|------|--------|
| `mi` | the current frame's entity referent with indexical `speaker` (normally `entity:1`) |
| `do`, `ko` | the current frame's entity referent with indexical `audience` (normally `entity:2`) — `ko` is the imperative form, same referent |
| `ti` | new referent, `category: indexical`, `indexical: proximalDemonstrative`, descriptor.kind `proSumti` |
| `ta` | indexical `medialDemonstrative` |
| `tu` | indexical `distalDemonstrative` |
| `da`/`de`/`di` | `category: variable`, descriptor.kind `proSumti` (bound by a quantified formula, see §3.4) |
| `zo'e` | an `elided` referent (descriptor.kind `elided`, word `zo'e`) |
| `zu'i` | descriptor.kind `typicalPlaceValue`, with `descriptor.speaker` set |
| `ke'a` | a relative-clause-head `parameter` (role `relativeClauseHead`) |
| `ce'u` | a property-slot `parameter` (role `propertySlot`) |
| `ma` | an argument-question `parameter` (role `argumentQuestion`) |
| `ti'u`/`di`-class utterance refs (`dei`,`di'u`,`de'u`,`da'u`,...) | descriptor.kind `utteranceReference` |
| other KOhA (ko'a, etc.) | `category: constant`, descriptor.kind `proSumti`, word = the cmavo (subject to anaphora resolution) |

The top-level frame normally uses `entity:1` speaker, `entity:2` audience,
`eventuality:3` now, and `entity:4` here. Top-level sibling utterances reuse
that frame; each parsed quotation gets a fresh frame. Seeing the same complete
id twice means the same deictic entity.

### 3.3 Logical/numeric quantifiers and scope

Quantification produces a `formula` whose `operator` is one of `exists`, `forall`, `none`,
`cardinality`, `pluralExists`, `pluralForall`, with `variable`/`restriction`/`body` and an optional
`quantity` link. Scope is expressed structurally: the **outer** quantifier's `body` is the
**inner** formula, so left-to-right Lojban quantifier order appears as nesting depth (outermost =
widest scope). Negation likewise wraps as a `not` formula at the appropriate nesting level.

Restrictions have operator-specific import behavior. On `exists`,
`pluralExists`, and `cardinality`, the restriction is conjoined with the body
at issue, so any witness consequence is already classical and no marker is
emitted. `none` is the classical no-witness reading and imports nothing. On
`forall` and `pluralForall`, a present restriction also commits to a nonempty
domain under CLL 16.8. That commitment survives contradictory negation, so the
formula carries `domainImport:"projective"`.

In particular, `not(forall restriction=R body=B domainImport=projective)` does
not negate the domain commitment. Moving the negation boundary across the
quantifier gives an existential with restriction `R` and body `not(B)`; that
existential entails the witness classically and therefore has no marker. A
bare universal such as `ro da zo'u da go broda gi brode` has no
`restriction`, so it has no `domainImport` and remains the import-free CLL
"any" contrast.

Quantifier word → operator/form:
- `ro` → operator `forall`, quantity form `all`.
- `no` → operator `none`.
- `su'o...` → operator `cardinality` (existential), quantity form `atLeast`.
- numeric (`pa`, `re`, `ci`, ...) → operator `cardinality`, quantity form `exact` (the count in
  `quantity.value.integer`).
- The `quantity` object carries `form` (see §2.12 list: also `atMost`/`moreThan`/`lessThan`/
  `approximate`/`enough`/`tooMany`/`tooFew` for me'i/za'u/ji'i/rau/du'e/mo'a etc.) and `scale`.

### 3.4 Bound variables da/de/di

A `da`/`de`/`di` produces a `variable` referent and a quantified `formula` binding it (typically
`exists`), with the rest of the sentence as `body`. Re-use of the same `da` within scope reuses
the same variable id.

### 3.5 SE conversion (se/te/ve/xe) — place remapping

`relation` is the **UNCONVERTED root selbri**, and the `arguments` keys are that **root
relation's canonical place numbers** (NOT the surface/post-conversion order). SE only decides
which surface sumti maps onto which root place: `se`↔x2, `te`↔x3, `ve`↔x4, `xe`↔x5 swap with x1.

So `mi se klama do` emits `relation: "klama"` with `arguments {x1: do, x2: mi, ...}`: `se` makes
the surface subject `mi` the *destination* (klama's x2), and `do` the *goer* (klama's x1), so each
filler is recorded under its **root-relation** key. **Reviewers: read `arguments` keys as the
underlying root-relation places — the converted filler is placed back into the root slot, not
left in surface order.** Stacked SE (e.g. `te se`) composes the swaps. (Same convention on
`ModalArgument.arguments`, §2.6.)

### 3.6 Abstractions (NU)

`nu`/`ka`/`du'u`/... produce a direct output referent; there is no public
`abstraction` wrapper object. Cmavo → output fields:

| Cmavo | output `sort` | content field |
|-------|---------------|---------------|
| `nu`   | `eventuality` | `content` |
| `mu'e` | `eventuality/achievement` | `content` |
| `pu'u` | `eventuality/process` | `content` |
| `zu'o` | `eventuality/activity` | `content` |
| `za'i` | `eventuality/state` | `content` |
| `ka`   | `relation`    | `body` |
| `ni`   | `amount`      | `body` |
| `jei`  | `truthValue`  | `body` |
| `du'u` | `proposition` | `body` |
| `si'o` | `concept`     | `body` |
| `li'i` | `eventuality/experience` | `content` |
| `su'u` | `abstractNature` | `body` |

For `ka`, `ce'u` slots become `parameters` and `arity` is their count.
Eventuality outputs are themselves the event used by inner predications and
carry their intensional `content`; other outputs carry `body`. `kau` questions
are retained in `embeddedQuestions`. No synthetic `eventOf`/`propertyOf`
predication or wrapper is emitted merely to connect the body to its output.

### 3.7 Tense, space, modal tags and `deicticGround`

Tense/space/modal information attaches to the **eventuality**, anchored against the utterance's
`deicticGround` (speech-time / here):

- **Temporal tense** (PU): `time` is an `AnchorRelation` whose `relation` is `"before"` (`pu`),
  `"at"` (`ca`), or `"after"` (`ba`); `anchor` points at the utterance's globally numbered
  `now` eventuality (normally `eventuality:3` in a simple graph). ZI distance adds `distance`
  ∈ `"short"`/`"medium"`/`"long"` (zi/za/zu). Ordered path steps, rather than the primary
  `AnchorRelation`, carry `introducedBy` provenance.
- **Spatial tense** (FAhA/VA): `space` is an `AnchorRelation`, with relation strings such as
  `"inFrontOf"` (ca'u), `"behind"` (ti'a), `"leftOf"` (zu'a), `"rightOf"` (ri'u), `"above"` (ga'u),
  `"below"` (ni'a), `"toward"` (fa'a), `"awayFrom"` (to'o), and many more; `anchor` defaults to
  the utterance's globally numbered `here` entity (normally `entity:4` in a simple graph).
- **Modals** (BAI / `fi'o`): become `modalArguments` on the predication (`relation` is the modal's
  name e.g. `"ki'u"`/`"mu'i"`, with its own numbered `arguments`).
- **Questioned tense** (`cu'e`): sets the eventuality's `tenseModal` to a `tenseQuestion`
  parameter.
- **Negated tense** uses `scalarNegation` on the relation; intervals/spans use `timeInterval`/
  `timeSpan`/`spaceInterval`; aspect (ZAhO) uses `aspect`/`aspects`.

`deicticGround` is set once per utterance to ordinary global ids such as
`{ "time": "eventuality:3", "place": "entity:4" }` and is what unanchored
tenses resolve against. The ids are allocated normally and are not symbolic
special values.

### 3.8 Relative clauses (poi / noi / voi / pe / po / ...)

Relative clauses appear in the head referent's `relativeClauses` array (or on an `ArgumentValue`),
each a `RelativeClause = { kind, body, introducedBy?, veridical?, source? }`:

- **`poi`** (NOI restrictive) → `kind: restrictive` — narrows which referent is meant.
- **`noi`** (NOI incidental) → `kind: incidental` — an aside, not truth-conditionally restrictive.
- **`voi` / `voi'e`** (nonveridical) → restrictive in kind but with `veridical: false` (the
  defining claim is not asserted true). Note: `veridical: true` is **never serialized** (the
  default), so absence of `veridical` means veridical.
- Relative **phrases** (GOI/`pe`-class): `pe`/`po`/`po'e`/`po'u` → `kind: restrictive`;
  `ne`/`no'u` → `kind: incidental`. Their `body` is a formula expressing the association relation
  (`"associatedWith"` for pe/ne, `"specificallyAssociatedWith"` for po, `"intrinsicallyPossessedBy"`
  for po'e, `"identity"` for po'u/no'u).

`body` is always a `formula` whose free variable is the head referent (`ke'a`).

### 3.9 Negation: `na` vs `na'e`

- **`na`** (bridi negation): wraps the bridi's formula in a `formula` with `operator: not`. (Prenex
  and selbri-level negations likewise produce `not`-formulas at the correct scope.)
- **`na'e` and contraries** (NAhE scalar negation): produces a `ScalarNegation` (on a predication's
  `scalarNegation`, or on a tense/anchor relation), with `kind`:
  `na'e` → `otherThan`, `to'e` → `opposite`, `no'e` → `neutral`, `je'a` → `affirmed`; `introducedBy`
  is the cmavo. This is *scalar/contrary* negation, not logical negation, and does **not** create a
  `not` formula.

### 3.10 Connectives: je / ja / .a / gi'e / .i je / ge...gi

All afterthought and forethought logical connectives become connective `formula`s (operators
`and`/`or`/`implies`/`iff`/`exclusiveOr`/`whetherOrNot`, or `connectiveQuestion` for `je'i`), with
the two operands as `children` and a `Connector` recording the surface form. Base mapping
(independent of grammatical locus — JA selbri/tanru, A sumti, GIhA bridi-tail, GA/GUhA forethought
all share it):

| Vowel class | operator |
|-------------|----------|
| `e` (je/.e/gi'e/ge) | `and` |
| `a` (ja/.a/gi'a/ga) | `or` |
| `o` (jo/.o/gi'o/go) | `iff` |
| `u` (ju/.u/gi'u/gu) | `whetherOrNot` |
| `na ja` / `na .a` (negated-left implication) | `implies` |

`na`/`nai` modifiers on the connective wrap the relevant side in a `not` formula. The
`connector.locus` records where it sat (`"sumti"`, `"selbri"`, `"tanru"`, `"bridi"`, `"tense"`),
and `connector.truthTable` preserves the exact surface connective text. Sentence connectives
(`.i je`) attach as `connectionClaims` on the enclosing `sequence`.

---

## 4. The `source` sub-object

Most objects (and several nested structs) carry a `source` of type `SemanticSource`:

```json
"source": {
  "span": { "byteStart": <int>, "byteEnd": <int> },
  "text": "<the source substring>",
  "construct": "<tag>"
}
```

- **`span`** — `{ byteStart, byteEnd }`, byte offsets into the original input (half-open range;
  `byteStart ≤ byteEnd`). When an object spans multiple source spans, `byteStart`/`byteEnd` are the
  min/max over them.
- **`text`** — the exact source substring `input[byteStart..byteEnd]` (omitted when unavailable).
- **`construct`** — a short tag naming the syntactic construct that produced this object. It is a
  free-form `Option<String>` in the model (no fixed enum), so the set below is *observed*, not
  exhaustively closed; new tags may appear. The tags observed in the builder include:

```
abstraction              abstraction-about            abstraction-body
abstraction-connection-formula  abstraction-description  assigned-name
bound-argument           bridi                        bridi-formula
bridi-negation           bridi-negation-boundary      bridi-tail-connection-claim
bridi-tail-connection-formula   bridi-tail-formula    compound-bridi-formula
connected-selbri-formula connected-sumti              connective-utterance
deleted-place            description                  distributed-formula
distributed-negation     distributed-predication      elided-place
elided-sumti             exact-magnitude              fragment
implicit-property-slot   indicator-expression         indicator-utterance
indirect-question        letteral                     math-expression
modal-argument           modal-branch-formula         modal-connection-formula
modal-indicator          name-words                   number-sumti
operand-connection-formula  parameter                 place-question
possessive-sumti         predication                  prenex-negation
quantifier-scope         quantity                     question
quotation                reciprocity                  relation-question-formula
relation-variable-formula  relative-clause            relative-phrase
restrictive-formula      restrictive-predication      restrictive-selbri-formula
restrictive-tanru-formula  selbri-operand             statement
statement-connection     statement-connection-claim   sumti
sumti-connection-claim   sumti-connection-formula     tanru-formula
tanru-inversion-formula  tense-modal                  tense-modal-fragment
tense-negation           tense-scope                  termset-connection-formula
text                     text-group-sequence          vocative
vocative-description     vocative-question
```

These tags indicate which grammar production created the object (e.g. `predication`, `sumti`,
`restrictive-formula` for a poi-body, `quantifier-scope` for a quantified formula's scope, etc.)
and are useful for correlating a JSON node back to a piece of the Lojban surface text.

---

## Appendix: complete enum value index (JSON spellings)

- **`type` (public JSON)**: utterance, sequence, referent, parameter, predication, formula,
  displayedContent, mathExpression, quantity, relationMetadata, question.
- **`force` / `UtteranceForce`**: assert, ask, command, mention, quote, parenthetical,
  subordinated, vocative.
- **`relation` / `SequenceRelation`** (kebab-case): same-topic-continuation; or tagged
  paragraph-boundary with transition new-topic/resume-prior-topic.
- **`actuality.kind` / `ActualityKind`**: actual, capable, potential, demonstrated.
- **`category` / `ReferentCategory`**: constant, variable, indexical, composite.
- **`sort` / `domain` / `SemanticSort`**: entity, mass, set, sequence, time, eventuality,
  eventuality/state, eventuality/process, eventuality/activity, eventuality/achievement,
  eventuality/experience, eventuality/locution, predication, truthValue, proposition, concept,
  amount, quantity, number, scale, text, sign, relation, place, connective, tenseModal,
  mathOperator, argumentBundle, abstractNature.
- **`indexical` / `IndexicalKind`**: speaker, audience, now, here, proximalDemonstrative,
  medialDemonstrative, distalDemonstrative.
- **`role` / `ParameterRole`**: propertySlot, relativeClauseHead, argumentQuestion,
  relationQuestion, relationVariable, unspecifiedRelation, placeQuestion, connectiveQuestion,
  tenseQuestion, mathOperatorQuestion, quantityQuestion, attitudeQuestion, respectiveSlot.
- **`ArgumentValue.kind` / `ArgumentValueKind`**: filled, elided, deleted.
- **`mode` / `PredicationMode`**: asserted, definitional, restrictive, incidental, displayed,
  inert, performative.
- **`scalarNegation.kind` / `ScalarNegationKind`**: otherThan, opposite, neutral, affirmed.
- **`operator` (formula) / `FormulaOperator`**: atom, affirmed, not, scoped, and, or, implies, iff,
  exclusiveOr, whetherOrNot, connectiveQuestion, exists, forall, none, cardinality, pluralExists,
  pluralForall, quantifierBundle, respectivelyDistribution.
- **`domainImport` / `DomainImport`**: projective.
- **`kind` (sign) / `SignKind`**: quotation, letteral, mathExpression, connective, word, text.
- **`LetteralUnit.kind` / `LetteralUnitKind`**: glyph, digit, shift, characterCode, compound.
- **`family` / `DisplayedContentFamily`**: emotion, attitudeModifier, propositionalAttitude,
  evidential, discursive, metalinguistic, emphasis, questionPrompt.
- **`polarity` / `DisplayedContentPolarity`**: positive, neutral, negative.
- **`assertionEffect` / `DisplayedContentAssertionEffect`**: none, hostAsserted, hostSubordinated,
  metalinguisticallyVoided, performative.
- **`form` / `QuantityForm`**: exact, all, atLeast, atMost, moreThan, lessThan, approximate,
  indefinite, enough, tooMany, tooFew.
- **`scale` / `QuantityScale`**: count, fraction, ordinal, amount, extent, frequency.
- **`kind` (question) / `QuestionKind`**: truth, argument, relation, place, connective, tense,
  mathOperator, attitude, quantity, multiple.
- **`mode` (question) / `QuestionMode`**: direct, indirect.
- **`QuestionSlot.role` / `QuestionSlotRole`**: answer, respectiveSlot.
- **`EndpointInclusion`**: inclusive, exclusive.
- **`RelativeClauseKind`**: incidental, restrictive.
- **`RecurrenceKind`**: occurrenceCount, ordinalOccurrence, regular, typically, continuously,
  habitually.
- **`RecurrenceConnectionKind`**: product.
- **`ModalNegationKind`**: contradictory.
- **`SpatialMotionKind`**: toward.
- **`TemporalPathAnchorKind`**: object, previous.
- **`severity` / `DiagnosticSeverity`**: info, warning, error.

---

## 5. Known divergences & do-not-flag (audited 2026-07-16)

This primer documents **what `tersmu` emits today**. The authoritative *design* spec
(what the model *should* be) is [`docs/semantic-model-spec.md`](../docs/semantic-model-spec.md)
— see its **“Amendments — jbotci CLL Review Pass”** and **“Known Implementation
Divergences”** sections, and the conceptual rationale in
[`docs/semantic-model-design.md`](../docs/semantic-model-design.md).

**Status: the chapters 9–11/14 review findings and the semantics-coverage
follow-ups have been fixed.**
The model was overhauled to implement the adopted amendments. **Review against the amended
spec** (`docs/semantic-model-spec.md`) and flag *genuine new* deviations. Do **not** re-file
the items below — they are resolved; they are listed only so you recognize the **now-correct**
shapes and the deliberate gaps.

**Resolved (now correct — recognize the new shapes, do not flag):**
- **Tanru** desugar with a typed `tanruLink` on the link predication:
  `{ head: <tertau predication id>, modifier: <ka/composite id>, relationLabel }` (no longer an
  opaque `R[tanru:…]` string) — #8.
- **`lo'e`/`le'e`** descriptor bodies now carry `veridical: false` (no longer identical to `lo`) — #9.
  (The *kind/archetype/genericity* structure is **deliberately not modeled** — the design exposes
  that gap on purpose; do **not** propose a `Kind` sort or a generic mode.)
- **`ROI`** recurrence now references a first-class `quantity` object (with `form`) and appears
  under `intervalModifiers` — #5, #3.
- **`ki`** sticky tense/space is serialized via a `sticky` marker — #6.
- **Stacked/interleaved aspect** operators are an ordered `intervalModifiers` stack
  (outermost-first) — #3.
- **whether-or-not (`U`)** marks the non-asserted operand `mode=inert`; **`SE`** on a connective
  exchanges operands structurally — #10, #12.
- **Negated logical connectives** canonicalize **uniformly** to the base-vowel operator
  (`or`/`and`/`iff`) with the negated operand wrapped in a `not` formula, at **every** locus
  (afterthought, sumti, bridi-tail, **and forethought incl. leading `ganai`/`gonai`/`gu'enai`**);
  `connector.source` records the surface `na`/`nai` — #14, #15, #22.
- **Connective runs** nest **binary**, surface-mirroring; never flattened across distinct
  operators — #18.
- **`.i joi`/`.i ce'o`** attach a truth-valueless `nonlogicalConnection` to the `sequence` — #7.
- **Connected operands** carry `force: subordinated`; **`fa'u`** uses a `respectivelyDistribution`
  node; **`ni`** fills its scale place; **`vau`**-shared ids are normative — #11, #4, #2, #20.
- Doc-only (always were correct): `quotation.mode` = category `parsed`/`opaque` (delimiter is a
  separate field) — #19; `abstractionAbout`/`NAhE`/`la'e`/`lu'e` are valid `descriptor.kind`
  values, `operand` is the general inner-object field — #21; `ModalArgument` uses SE-remapped
  place numbers — #17.
- **Fragments and text edges** retain typed denotations, structured relation
  labels, `elidedConnectionOperand`, and paragraph-transition values; no empty
  bridi is invented (PR #384).
- **Composed questions** use `kind:multiple` with ordered typed slots;
  quantity values retain `quantityQuestion` parameters, and truth slots have
  no parameter (PR #414).
- **Abstraction outputs** are direct referents, and derived trees descend into
  reachable eventuality `content`; there is no public `abstraction` wrapper
  object (PR #417).
- **Unsupported-path retirement** is complete: valid standard constructs are
  structurally lowered, undefined experimental semantics and missing discourse
  context receive principled errors, and there is no `unsupported`
  disposition/placeholder in the graph (PR #416).
- **Linked-unit currying and grouped conversion** use the exposed-place frame
  documented in the spec; `be fa` occupancy and outer `se`/`te`/`ve`/`xe`
  conversions must not be compacted or silently discarded (PRs #441/#446).

**Still genuinely open:** no accepted model/encoding divergence is currently
listed in the normative spec. Flag a new mismatch; do not turn a product bug
into a do-not-flag exception.

**Open product bugs found by the current audit:** followed NIhO paragraph
transitions are dropped (#447); non-quantified `zo'u` topics are discarded
(#448); ordinary lujvo omit the specified relation metadata (#450); and
implicit-abstraction lujvo omit their implicit event content (#451). These are
descriptions of current output for reviewers, not accepted normative
exceptions.

**Snapshot freshness:** `out/` snapshots are **regenerated from the current binary per chapter
run**, so they are not stale. (Historical note: the discourse-deictic-duplication bug was fixed
in `f53ff7dcb9`; the current binary shares the same globally numbered frame
referents across sibling top-level utterances — never expect symbolic special ids.)
