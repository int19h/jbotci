# `lojban-semantics-json-1` — tersmu JSON Reference

This is the authoritative reference for the JSON that `jbotci tersmu --format json` emits.
It is generated from the Rust model in `crates/jbotci-semantics/src/model.rs` (the structural
source of truth) and the builder in `crates/jbotci-semantics/src/builder.rs` (which maps Lojban
syntax onto these objects). Use it to read a tersmu semantic graph and judge whether it correctly
captures the meaning of a Lojban utterance.

All field names below are the **exact JSON keys** (the model serializes with serde
`rename_all = "camelCase"` unless noted, so Rust `byte_start` → JSON `byteStart`, etc.). All enum
values are given exactly as they serialize (almost all enums are `camelCase`; a few are
`kebab-case` and are flagged). Optional fields are **omitted** when empty/`None` (serde
`skip_serializing_if`); never assume a missing key means a different value than "absent."

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
  `utterance:u1`, a `sequence:s1` (multiple top-level utterances), or for a bare fragment a
  `formula:f1`.
- **`objects`** — a map from **id string** to object. Every object referenced anywhere in the
  graph is defined here (the graph is validated to have no dangling references), and `root` is
  always a key in this map.

### Object id conventions

Ids are strings of the form `«kind»:«prefix»«index»`. The kind word before the colon and the
prefix letter both encode the object's `type`. The validator enforces that the id prefix matches
the object's `type` field. Indices are 1-based and assigned in build order; the index alone is not
semantically meaningful, but identity (same id used twice) **is** — it means the same object.

| Id form           | `type`             | Notes |
|-------------------|--------------------|-------|
| `utterance:u«n»`  | `utterance`        | |
| `sequence:s«n»`   | `sequence`         | |
| `eventuality:e«n»`| `eventuality`      | index may be 0 |
| `referent:r«n»`   | `referent`         | ordinary referents |
| `referent:speaker`, `referent:addressee`, `referent:speech-time`, `referent:here` | `referent` | the four **special deictic referents** (note: kebab-case `speech-time`); shared/singleton across the graph |
| `parameter:p«n»`  | `parameter`        | |
| `predication:p«n»`| `predication`      | (prefix `p`, like parameter, but kind word `predication`) |
| `formula:f«n»`    | `formula`          | |
| `abstraction:a«n»`| `abstraction`      | |
| `sign:s«n»`       | `sign`             | (kind word `sign`) |
| `display:d«n»`    | `displayedContent` | note the id word is `display`, the `type` is `displayedContent` |
| `math:m«n»`       | `mathExpression`   | |
| `quantity:q«n»`   | `quantity`         | |
| `relation:r«n»`   | `relationMetadata` | (kind word `relation`) |
| `question:q«n»`   | `question`         | |

The set of `type` values (serde-`camelCase` of `SemanticObjectKind`) is exactly:
`utterance`, `sequence`, `eventuality`, `referent`, `parameter`, `predication`, `formula`,
`abstraction`, `sign`, `displayedContent`, `mathExpression`, `quantity`, `relationMetadata`,
`question`.

---

## 2. Object types and their fields

Every object is serialized from one flat Rust struct (`SemanticObject`); each `type` populates a
characteristic subset of fields and leaves the rest absent. The `type` field is always present
(serialized under the JSON key **`type`**). Below, each type lists the fields that are meaningful
for it. Two fields are universal:

- **`source`** — optional `SemanticSource` (see §4); provenance/span. Present on most objects.
- **`diagnostics`** — optional array of `{ "severity": ..., "message": ... }`; `severity` ∈
  `info` | `warning` | `error` (currently always `warning` in practice). Omitted when empty.

The full inventory of fields and their enum domains follows by object type. Field keys not listed
under a type may still appear if the builder reuses them, but the per-type lists below cover the
intended population.

### 2.1 `utterance`

A complete speech act.

| Key | Type | Meaning |
|-----|------|---------|
| `force` | `UtteranceForce` | illocutionary force; one of `assert`, `ask`, `command`, `mention`, `quote`, `parenthetical`, `vocative` |
| `speaker` | id → `referent` | the speaker; normally `referent:speaker` |
| `audience` | id → `referent` | the addressee; normally `referent:addressee` |
| `eventuality` | id → `eventuality` | the locution eventuality of this utterance |
| `content` | id | the asserted content. For ordinary utterances a `formula`, `sequence`, or `question`. For `force = mention`, may instead be any argument-fillable object (referent/sign/etc.) — a mentioned thing rather than a claim |
| `deicticGround` | `DeicticGround` | `{ "time": "referent:speech-time", "place": "referent:here" }`; the deictic origin |
| `asides` | array of id → `utterance` | parenthetical/vocative side-utterances attached here |
| `vocativeKind` | string | present for vocative utterances |

`deicticGround` is set on every utterance the builder produces (to speech-time/here), establishing
the anchors that tense/space relations resolve against.

### 2.2 `sequence`

An ordered grouping of top-level utterances/sequences (e.g. several `.i`-joined statements, or a
text with multiple paragraphs).

| Key | Type | Meaning |
|-----|------|---------|
| `items` | array of id | members, each an `utterance`, `sequence`, or `displayedContent` |
| `relation` | `SequenceRelation` | serialized from `sequence_relation`; currently only `same-topic-continuation` (**kebab-case**) |
| `connectionClaims` | array of id → `formula` | formulas asserting the logical connection between consecutive items (for `.i je`-style sentence connectives) |
| `content` | id → `formula` | when the whole sequence carries a single combined claim |

### 2.3 `eventuality`

The event/state/process introduced by a bridi (and the locution event of an utterance). Tense,
aspect, and spatial information hang off the eventuality.

| Key | Type | Meaning |
|-----|------|---------|
| `class` | `EventualityClass` | `locution`, `event`, `state`, `process`, `activity`, `achievement`. `locution` is the speech-act event of an utterance |
| `actuality` | `Actuality` = `{ "kind": ActualityKind }` | `kind` ∈ `actual`, `capable`, `potential`, `demonstrated` (from CAhA: ca'a/ka'e/nu'o/pu'i) |
| `content` | id | the formula or sequence this eventuality is the occurrence of |
| `tenseModal` | id → `parameter` | present when the tense itself is questioned (`cu'e`); the parameter has sort `tenseModal`, role `tenseQuestion` |
| `time` | `AnchorRelation` | primary temporal placement (see §3.7) |
| `timePath` | array of `TemporalPathStep` | chained temporal offsets |
| `timeInterval` | `TimeInterval` = `{ extent, anchor? }` | ZEhA-style duration |
| `timeSpan` | `TimeSpan` = `{ start, end, introducedBy }` | bounded span (each endpoint `{ relation, anchor?, introducedBy, distance?, scalarNegation? }`) |
| `aspect` / `aspects` | `Aspect` = `{ contour, anchor?, scalarNegation? }` | ZAhO event contour(s) |
| `recurrence` | array of `Recurrence` | ROI/TAhE/etc. repetition; see §2.x note below |
| `space` | `AnchorRelation` | primary spatial placement |
| `spacePath` | array of `TemporalPathStep` | chained spatial offsets |
| `spaceInterval` | `SpaceInterval` = `{ extent?, directions[], dimensions[], anchor? }` | VEhA/VIhA |
| `spatialAspect` / `spatialAspects` | `Aspect` | spatial contour |
| `spatialRecurrence` | array of `Recurrence` | |

`Recurrence` fields: `kind` ∈ `occurrenceCount`, `ordinalOccurrence`, `regular`, `typically`,
`continuously`, `habitually`; plus `introducedBy`, optional `connection`
(`{kind:"product", introducedBy}`), `value` (a `QuantityValue`), `interval` (id), `negation`
(`ModalNegation` `{kind:"contradictory", introducedBy}`).

### 2.4 `referent`

A thing that can fill an argument place — the semantic value of a sumti.

| Key | Type | Meaning |
|-----|------|---------|
| `category` | `ReferentCategory` | `constant`, `variable`, `indexical`, `composite` |
| `sort` | `SemanticSort` | semantic sort (see full list below) |
| `indexical` | `IndexicalKind` | present when `category = indexical`: `speaker`, `audience`, `speechTime`, `here`, `proximalDemonstrative`, `medialDemonstrative`, `distalDemonstrative` |
| `descriptor` | `Descriptor` | how this referent was described (le/lo/la/pro-sumti/etc.); see §3.1 |
| `composition` | `Composition` | for `category = composite` (set/mass/connected/interval sumti); see below |
| `relativeClauses` | array of `RelativeClause` | poi/noi attached to the referent (see §3.8) |
| `assignedNames` | array of `AssignedName` | goi/cei name assignments: `{ name, word, introducedBy, source? }` |

**`ReferentCategory`** values: `constant` (a fixed individual, e.g. names, `lo`-descriptions,
plain pro-sumti), `variable` (bound logical variable da/de/di), `indexical` (mi/do/ti/ta/tu and
deictic referents), `composite` (built from members via `composition`).

**`SemanticSort`** (the `sort` field, also used for `domain`): `entity`, `mass`, `set`,
`sequence`, `eventuality`, `predication`, `truthValue`, `proposition`, `concept`, `amount`,
`quantity`, `number`, `text`, `sign`, `relation`, `place`, `connective`, `tenseModal`,
`mathOperator`, `argumentBundle`.

**`Descriptor`** struct keys: `kind` (string — see the enumerated values in §3.1), `word` (the
source cmavo, e.g. `"le"`, `"lo"`, `"la"`, `"ti"`), optional `speaker` (id → referent, used by
speaker-relative descriptions), optional `body` (id → formula: the selbri restriction, e.g. for
`lo broda` the formula `broda(this)`), `relativeClauses`, optional `quantity` (id → quantity),
optional `name` (string, for `la`-names), optional `operand` (id — the inner object a
descriptor wraps/reuses; **not** only `li`/math operands: also the raised sumti of a
`tu'a`/`jai` raising (`descriptor.kind: abstractionAbout`), the `la'e`/`lu'e`
reference↔referent shift target, and the base referent of a `NAhE`/qualifier
(`otherThan`/`oppositeOf`/`neutralOf`)).

**`Composition`** struct keys: `operator` (string — e.g. `"set"`, `"mass"`, an `"...Interval"`
operator, `"and"`/`"or"` style connectives, or `"connectiveQuestion"`), optional
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
| `role` | `ParameterRole` | `propertySlot`, `relativeClauseHead`, `argumentQuestion`, `relationQuestion`, `relationVariable`, `placeQuestion`, `connectiveQuestion`, `tenseQuestion`, `mathOperatorQuestion`, `attitudeQuestion` |
| `introducedBy` | string | the source cmavo (e.g. `"ma"`, `"ce'u"`, `"ke'a"`, `"cu'e"`) |

The validator enforces sort/role coherence: e.g. `relationQuestion`/`relationVariable` ⇒ sort
`relation`; `placeQuestion` ⇒ sort `place`; `connectiveQuestion` ⇒ sort `connective`;
`tenseQuestion` ⇒ sort `tenseModal`; `mathOperatorQuestion` ⇒ sort `mathOperator`;
`argumentQuestion`/`relativeClauseHead`/`attitudeQuestion` ⇒ sort `entity`; `propertySlot` ⇒ any
sort.

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

### 2.8 `abstraction`

The semantic object for a NU-abstraction (nu/ka/du'u/...).

| Key | Type | Meaning |
|-----|------|---------|
| `abstractionKind` | `AbstractionKind` | `event`, `achievement`, `process`, `activity`, `state`, `property`, `amount`, `truthValue`, `proposition`, `sentenceSign`, `concept`, `experience`, `unspecified` |
| `body` | id → formula | the abstracted formula |
| `parameters` | array of id → parameter | the abstracted slots (e.g. `ce'u` slots for `ka`) |
| `arity` | int | set for `property` (= number of parameters) |

See §3.6 for the cmavo→`abstractionKind` mapping.

### 2.9 `sign`

A metalinguistic sign: a quotation, letteral string, math expression as text, connective word,
single word, or text.

| Key | Type | Meaning |
|-----|------|---------|
| `kind` | `SignKind` | serialized from `sign_kind`: `quotation`, `letteral`, `mathExpression`, `connective`, `word`, `text` |
| `text` | string | literal text for non-quotation signs |
| `letterals` | array of `LetteralUnit` | for letteral strings (BY etc.); see below |
| `quotation` | `Quotation` | for quotations: `{ mode, utterance?, delimiter?, text? }`. **`mode` is the structural category, not the delimiter cmavo**: `parsed` (structured `lu…li'u` — an `utterance` id is reachable from outside) or `opaque` (sealed `zo`/`lo'u`/`zoi` — no reachable referents). The surface delimiter is the separate `delimiter` field; `text` holds the raw source. |
| `denotes` | id | what the sign denotes (referent or mathExpression) |

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
| `assertionEffect` | `DisplayedContentAssertionEffect` | `none`, `hostAsserted`, `hostSubordinated`, `performative` |
| `intensity` | string | intensity word (CAI: cai/sai/ru'e/...) |
| `phase` | string | |
| `experiencer` | id → referent | who holds the attitude (usually speaker) |
| `target` | id | what it applies to (an utterance, or argument-fillable object) |
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
| `denotes` | id | for `mo'e`/`ni'e` operands: the referent or selbri-derived object the operand denotes |
| `endpointInclusion` | `IntervalEndpointInclusion` = `{ left, right }` | each ∈ `inclusive`/`exclusive`; only for `"...Interval"` operators |

### 2.12 `quantity`

A quantifier/number value with a form and scale.

| Key | Type | Meaning |
|-----|------|---------|
| `form` | `QuantityForm` | `exact`, `all`, `atLeast`, `atMost`, `moreThan`, `lessThan`, `approximate`, `indefinite`, `enough`, `tooMany`, `tooFew` |
| `value` | `QuantityValue` | exactly one of `integer` (int), `text` (string), or `mathExpression` (id) is present |
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

### 2.14 `question`

An explicit question abstraction (truth questions, fill-in questions, etc.).

| Key | Type | Meaning |
|-----|------|---------|
| `kind` | `QuestionKind` | serialized from `question_kind`: `truth`, `argument`, `relation`, `place`, `connective`, `tense`, `mathOperator`, `attitude`, `quantity` |
| `mode` | `QuestionMode` | serialized from `question_mode`: `direct`, `indirect` |
| `domain` | `SemanticSort` | the sort being asked about |
| `body` | id → formula | the question's open formula |
| `slots` | array of `QuestionSlot` = `{ parameter, role }` | the questioned positions; `role` ∈ `answer` |
| `asker` | id → referent | usually the speaker |
| `respondent` | id → referent | usually the addressee |
| `focus` | id → parameter \| referent | optional focused element |
| `presupposedAnswer` | id → parameter \| referent | optional presupposition |
| `embeddedQuestions` | array of id → question | nested indirect questions |

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

The table above lists the **article-bearing** kinds. `descriptor.kind` also takes
**non-article** values for descriptors built by other constructs — these are valid, not
defects: `abstractionAbout` (a `tu'a`/`jai` raising; carries `operand` = the raised sumti),
`otherThan`/`oppositeOf`/`neutralOf` (`NAhE` scalar qualifiers over `operand`), the
`la'e`/`lu'e` reference↔referent shifts, `proSumti` (KOhA / bound variables),
`elided` (`zo'e`), `typicalPlaceValue` (`zu'i`), and `utteranceReference` (`di'u`-series).
This is an open set; do not assume a referent is mis-built merely because its
`descriptor.kind` is not in the article table.

Reviewer expectations:
- **`lo broda`** — `category: constant`, `descriptor.kind: veridicalDescription`, `descriptor.word: "lo"`,
  `descriptor.body` → a formula `broda(r)` (the restriction). The referent is the veridical thing(s)
  that *are* broda.
- **`le broda`** — `descriptor.kind: speakerDescription`; the speaker's intended referent(s),
  described as broda but not asserted to be.
- **`la broda` / `la .cmevla.`** — `descriptor.kind: name`; for cmevla names the `descriptor.name`
  string and/or `assignedNames` carry the name; the body need not be a veridical claim.
- When the selbri is itself a NU-abstraction (e.g. `lo nu broda`), `descriptor.body` is an
  abstraction-description formula and the referent's `sort` follows the abstraction's output sort
  (eventuality/proposition/etc.).

### 3.2 Pro-sumti: `mi`, `do`, `ti`, `ta`, `tu`, da/de/di, zo'e, etc.

| Word | Result |
|------|--------|
| `mi` | the shared `referent:speaker` (indexical `speaker`) |
| `do`, `ko` | the shared `referent:addressee` (indexical `audience`) — `ko` is the imperative form, same referent |
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

The four indexical/deictic singletons `referent:speaker`, `referent:addressee`,
`referent:speech-time`, `referent:here` are reused everywhere; seeing the same id twice means the
same deictic entity.

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

`nu`/`ka`/`du'u`/... produce an `abstraction` object. Cmavo → `abstractionKind`:

| Cmavo | `abstractionKind` | output `sort` | link relation |
|-------|-------------------|---------------|---------------|
| `nu`   | `event`       | `eventuality` | `eventOf` |
| `mu'e` | `achievement` | `eventuality` | `achievementOf` |
| `pu'u` | `process`     | `eventuality` | `processOf` |
| `zu'o` | `activity`    | `eventuality` | `activityOf` |
| `za'i` | `state`       | `eventuality` | `stateOf` |
| `ka`   | `property`    | `relation`    | `propertyOf` |
| `ni`   | `amount`      | `amount`      | `amountOf` |
| `jei`  | `truthValue`  | `truthValue`  | `truthValueOf` |
| `du'u` | `proposition` | `proposition` | `propositionOf` |
| `si'o` | `concept`     | `concept`     | `conceptOf` |
| `li'i` | `experience`  | `eventuality` | `experienceOf` |
| (other) | `unspecified` | `entity`     | `abstractionOf` |

The abstraction's `body` is the inner formula; the inner predication's `mode` is `restrictive`
for `ka` (Property) and `inert` for the others. For `ka`, the `ce'u` slots become `parameters`
and `arity` is set to their count. Abstractions that yield eventualities are linked to an
eventuality via the relation name above; when used as a description body, the surrounding referent
takes the abstraction's output sort.

### 3.7 Tense, space, modal tags and `deicticGround`

Tense/space/modal information attaches to the **eventuality**, anchored against the utterance's
`deicticGround` (speech-time / here):

- **Temporal tense** (PU): `time` is an `AnchorRelation` whose `relation` is `"before"` (`pu`),
  `"at"` (`ca`), or `"after"` (`ba`); `anchor` defaults to `referent:speech-time`; `introducedBy`
  records the cmavo. ZI distance adds `distance` ∈ `"short"`/`"medium"`/`"long"` (zi/za/zu).
- **Spatial tense** (FAhA/VA): `space` is an `AnchorRelation`, with relation strings such as
  `"inFrontOf"` (ca'u), `"behind"` (ti'a), `"leftOf"` (zu'a), `"rightOf"` (ri'u), `"above"` (ga'u),
  `"below"` (ni'a), `"toward"` (fa'a), `"awayFrom"` (to'o), and many more; `anchor` defaults to
  `referent:here`.
- **Modals** (BAI / `fi'o`): become `modalArguments` on the predication (`relation` is the modal's
  name e.g. `"ki'u"`/`"mu'i"`, with its own numbered `arguments`).
- **Questioned tense** (`cu'e`): sets the eventuality's `tenseModal` to a `tenseQuestion`
  parameter.
- **Negated tense** uses `scalarNegation` on the relation; intervals/spans use `timeInterval`/
  `timeSpan`/`spaceInterval`; aspect (ZAhO) uses `aspect`/`aspects`.

`deicticGround` is set once per utterance to
`{ "time": "referent:speech-time", "place": "referent:here" }` and is what unanchored tenses
resolve against.

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

- **`type` / `SemanticObjectKind`** (camelCase): utterance, sequence, eventuality, referent,
  parameter, predication, formula, abstraction, sign, displayedContent, mathExpression, quantity,
  relationMetadata, question.
- **`force` / `UtteranceForce`**: assert, ask, command, mention, quote, parenthetical, vocative.
- **`relation` / `SequenceRelation`** (kebab-case): same-topic-continuation.
- **`class` / `EventualityClass`**: locution, event, state, process, activity, achievement.
- **`actuality.kind` / `ActualityKind`**: actual, capable, potential, demonstrated.
- **`category` / `ReferentCategory`**: constant, variable, indexical, composite.
- **`sort` / `domain` / `SemanticSort`**: entity, mass, set, sequence, eventuality, predication,
  truthValue, proposition, concept, amount, quantity, number, text, sign, relation, place,
  connective, tenseModal, mathOperator, argumentBundle.
- **`indexical` / `IndexicalKind`**: speaker, audience, speechTime, here, proximalDemonstrative,
  medialDemonstrative, distalDemonstrative.
- **`role` / `ParameterRole`**: propertySlot, relativeClauseHead, argumentQuestion,
  relationQuestion, relationVariable, placeQuestion, connectiveQuestion, tenseQuestion,
  mathOperatorQuestion, attitudeQuestion.
- **`ArgumentValue.kind` / `ArgumentValueKind`**: filled, elided, deleted.
- **`mode` / `PredicationMode`**: asserted, definitional, restrictive, incidental, displayed,
  inert, performative.
- **`scalarNegation.kind` / `ScalarNegationKind`**: otherThan, opposite, neutral, affirmed.
- **`operator` (formula) / `FormulaOperator`**: atom, not, scoped, and, or, implies, iff,
  exclusiveOr, whetherOrNot, connectiveQuestion, exists, forall, none, cardinality, pluralExists,
  pluralForall. (For `mathExpression`, `operator` is instead a free-form string.)
- **`domainImport` / `DomainImport`**: projective.
- **`abstractionKind` / `AbstractionKind`**: event, achievement, process, activity, state,
  property, amount, truthValue, proposition, sentenceSign, concept, experience, unspecified.
- **`kind` (sign) / `SignKind`**: quotation, letteral, mathExpression, connective, word, text.
- **`LetteralUnit.kind` / `LetteralUnitKind`**: glyph, digit, shift, characterCode, compound.
- **`family` / `DisplayedContentFamily`**: emotion, attitudeModifier, propositionalAttitude,
  evidential, discursive, metalinguistic, emphasis, questionPrompt.
- **`polarity` / `DisplayedContentPolarity`**: positive, neutral, negative.
- **`assertionEffect` / `DisplayedContentAssertionEffect`**: none, hostAsserted, hostSubordinated,
  performative.
- **`form` / `QuantityForm`**: exact, all, atLeast, atMost, moreThan, lessThan, approximate,
  indefinite, enough, tooMany, tooFew.
- **`scale` / `QuantityScale`**: count, fraction, ordinal, amount, extent, frequency.
- **`kind` (question) / `QuestionKind`**: truth, argument, relation, place, connective, tense,
  mathOperator, attitude, quantity.
- **`mode` (question) / `QuestionMode`**: direct, indirect.
- **`QuestionSlot.role` / `QuestionSlotRole`**: answer.
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

## 5. Known divergences & do-not-flag (2026-06-23)

This primer documents **what `tersmu` emits today**. The authoritative *design* spec
(what the model *should* be) is [`docs/semantic-model-spec.md`](../docs/semantic-model-spec.md)
— see its **“Amendments — jbotci CLL Review Pass”** and **“Known Implementation
Divergences”** sections, and the conceptual rationale in
[`docs/semantic-model-design.md`](../docs/semantic-model-design.md).

**Status (updated 2026-06-24): the chapters 9–11/14 review findings have been FIXED.**
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

**Still genuinely open (flag if you hit them, reference the issue):** the *spec* extensions
that were adopted but may not be fully realized — consult the open issues; and any construct not
exercised by chapters 9–11/14.

**Snapshot freshness:** `out/` snapshots are **regenerated from the current binary per chapter
run**, so they are not stale. (Historical note: the discourse-deictic-duplication bug was fixed
in `f53ff7dcb9`; the current binary shares the `referent:speaker`/`referent:addressee` singletons
across utterances — never flag duplicated indexical referents.)
