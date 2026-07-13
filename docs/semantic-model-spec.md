# Lojban Discourse Object Model in JSON

> **jbotci note.** This is the authoritative format specification for the
> `lojban-semantics-json-1` graph emitted by `jbotci tersmu`. It was imported
> into the repository from the `sem` design notes; its conceptual companion
> (the Part 0.A–0.P design rationale + authoritative changelog) lives at
> [`semantic-model-design.md`](semantic-model-design.md) and worked CLL
> encodings at [`semantic-model-cll-encodings.md`](semantic-model-cll-encodings.md).
> Two repository-specific sections are appended at the end:
> **“Amendments — jbotci CLL Review Pass (2026-06-23)”** (decisions adopted from
> the chapters 9–11/14 review, each citing its tracking issue) and
> **“Known Implementation Divergences”** (where current `tersmu` output departs
> from this spec). The agent/consumer-facing field cheat-sheet used by the
> review harness is `review/tersmu_schema_primer.md` — kept deliberately
> separate (see the divergences section for why).

This document restates the semantic object model from
[`semantic-model-design.md`](semantic-model-design.md) in JSON terms and records
the object-model amendments suggested by reviewing the v0 `tersmu` Lean prelude
and generated outputs.

The goal is still a semantic object graph, not a pretty notation.  JSON is used
here because the first concrete output format should be easy to consume,
validate, diff, and render in multiple UIs.

## Design Goals

1. Use English object and attribute names where possible.  Lojban words remain
   appropriate as lexical relation names, surface cmavo labels, or source text.
2. Keep semantic objects distinct from parser nodes.  A source span or source
   token may be attached as provenance, but it is not the object identity.
3. Preserve distinctions that matter semantically, even when v0 `tersmu`
   represented them only indirectly through Lean helpers.
4. Avoid using Lean concepts as the core model.  The graph should be renderable
   to JSON, Lean, explanation text, search indexes, or UI trees.
5. Make unresolved interpretation visible.  Ambiguity, anaphora resolution, and
   unsupported constructs should not be hidden by fallback objects that look
   fully interpreted.

## Envelope

The top-level value is an object graph.  IDs are keys in `objects`; object
values refer to other objects by those IDs.  Public output contains only
objects reachable from `root` by following semantic ID fields; builder-only
temporaries and abandoned helper objects must be pruned before serialization.

```json
{
  "version": "lojban-semantics-json-1",
  "root": "utterance:5",
  "objects": {
    "entity:1": {
      "type": "referent",
      "sort": "entity",
      "category": "indexical",
      "indexical": "speaker"
    },
    "entity:2": {
      "type": "referent",
      "sort": "entity",
      "category": "indexical",
      "indexical": "audience"
    },
    "eventuality:3": {
      "type": "referent",
      "sort": "eventuality",
      "category": "indexical",
      "indexical": "now"
    },
    "entity:4": {
      "type": "referent",
      "sort": "entity",
      "category": "indexical",
      "indexical": "here"
    },
    "utterance:5": {
      "type": "utterance",
      "force": "assert",
      "speaker": "entity:1",
      "audience": "entity:2",
      "eventuality": "eventuality/locution:6",
      "content": "formula:7",
      "deicticGround": {
        "time": "eventuality:3",
        "place": "entity:4"
      }
    }
  }
}
```

IDs should be stable within one output but need not be stable across different
parser versions.  Public IDs are opaque but intentionally readable.  The
numeric suffix is globally unique across the entire graph; `entity:1` and
`predication:1` must not both appear.  For referents, the prefix is the
serialized semantic sort path.  For structural objects, the prefix is the
object kind.

Referent prefixes include:

| Prefix | Referent sort |
| --- | --- |
| `entity:` | `entity` |
| `eventuality:` | broad `eventuality` (`nu` and bare bridi events) |
| `eventuality/state:` | `za'i` state |
| `eventuality/process:` | `pu'u` process |
| `eventuality/activity:` | `zu'o` activity |
| `eventuality/achievement:` | `mu'e` achievement |
| `eventuality/experience:` | `li'i` experience |
| `eventuality/locution:` | speech/text act event |
| `relation:` | relation/property |
| `proposition:` | proposition |
| `truthValue:` | truth value |
| `amount:` | amount |
| `number:` | number |
| `scale:` | scale |
| `sign:` | sign |
| `abstractNature:` | `su'u` abstract nature |
| `concept:` | concept |

Structural prefixes include `utterance:`, `sequence:`, `predication:`,
`formula:`, `quantity:`, `math:`, `parameter:`, `question:`, `display:`, and
`relationMetadata:`.

Top-level text starts with one shared deictic frame: `entity:1` is the speaker,
`entity:2` is the audience, `eventuality:3` is now, and `entity:4` is here.
Sibling top-level `.i` utterances share that full frame.  Parsed quotations
allocate a fresh speaker, audience, now, and here; inner `mi`/`do`, tense
anchors, spatial anchors, and demonstratives use the quoted frame.

## Common Fields

Every object has a required `type`.  Other common fields are optional.

```json
{
  "type": "predication",
  "source": {
    "text": "klama",
    "span": { "byteStart": 3, "byteEnd": 8 },
    "tokens": ["token:t2"],
    "construct": "selbri"
  },
  "labels": ["main predicate"],
  "diagnostics": []
}
```

`source` is provenance, not semantics.  It is useful for explanation UIs,
debugging, and expectation diffs, but consumers should not need it to determine
truth conditions.

`diagnostics` is for lossy, incomplete, or disputed interpretation.  A renderer
should be able to show an object graph with diagnostics rather than inventing
a semantic placeholder that pretends to be complete.

## Object Types

### utterance

An utterance is a speech or text act.  It has a speaker, an audience, a
locution eventuality, and content.

```json
{
  "type": "utterance",
  "force": "assert",
  "speaker": "entity:1",
  "audience": "entity:2",
  "eventuality": "eventuality/locution:6",
  "content": "formula:1000",
  "deicticGround": {
    "time": "eventuality:3",
    "place": "entity:4"
  },
  "asides": ["utterance:1001", "display:1002"]
}
```

`asides` may contain nested vocative/parenthetical utterances and displayed
content anchored to this utterance.  Displayed content is kept in `asides`
when it comments on a host formula, referent, or discourse act but is not
itself the utterance `content`.

`force` values include:

- `assert`
- `ask`
- `command`
- `mention`
- `quote`
- `parenthetical`
- `subordinated`
- `vocative`

`subordinated` is used for surface utterance items whose truth is carried by
a single combined formula.  For example, `.ije` creates one asserted connected
formula on the sequence; the two sequence items are retained for discourse
order and provenance, but their force is `subordinated` so consumers do not
double-assert the operands.

For vocatives, use `vocativeKind` when known:

```json
{
  "type": "utterance",
  "force": "vocative",
  "vocativeKind": "greeting",
  "speaker": "entity:1",
  "audience": "entity:1003",
  "eventuality": "eventuality/locution:12",
  "content": null
}
```

When a vocative appears inside another utterance, preserve it as a vocative
utterance in the enclosing utterance's `asides`.  Its `audience` is the named or
described addressee when the vocative supplies one.  Self-identification
vocatives such as `mi'e .djan.` use the introduced name referent as their
`content` and set that referent's `target` to the current speaker referent.
Bare-selbri
vocatives such as `coi xunre pastu nixli` target an implicit speaker
description, equivalent in force to `coi le xunre pastu nixli`.  The vocative
selbri is still a full selbri: tanru, linkargs, conversion, scalar negation, and
other selbri structure are lowered through the ordinary selbri-body path, not
flattened to a text label.

The v0 prelude treated quotations as functions of speaker and addressee.  The
JSON model keeps that insight by making quoted text a nested utterance with its
own speaker/audience slots, not a flattened string unless the source is opaque.

### sequence

A sequence is discourse organization.  Plain `.i`, `ni'o`, paragraphs, and
`tu'e ... tu'u` blocks belong here unless a connective explicitly creates a
truth-functional formula.

```json
{
  "type": "sequence",
  "items": ["utterance:1004", "utterance:1005"],
  "content": "formula:1006",
  "connectionClaims": ["formula:1007"],
  "ordinalLabels": [
    {
      "target": "utterance:1004",
      "level": "item",
      "value": "math:1008",
      "introducedBy": "mai"
    }
  ],
  "relation": "same-topic-continuation"
}
```

Plain discourse sequencing has no truth value.  This intentionally differs
from a Lean-oriented rendering that can make adjacent propositions appear as a
single conjunction for typechecking convenience.  If `.ije` or another
explicit statement logical connector is present, keep the sequence for the
utterance acts but put the truth-functional formula in `content`.  Sequence
`items` may contain utterances, nested sequences, or displayed-content objects.
Displayed-content items are only for non-truth-valued mention content such as
an indicator-only utterance with multiple independent attitudes; ordinary
discourse sequencing still uses utterance or sequence items.  Nested sequences
are needed for grouped discourse or left-associated multi-statement text such as
`A .ije B .ije C`, where preserving the discourse grouping is preferable to
pretending that the sequence itself is an utterance.

Numerical free modifiers headed by `mai`/`mo'o` are represented as
`ordinalLabels` on the containing sequence.  `target`, when present, points at
the item being labelled.  `level:"item"` corresponds to `mai`;
`level:"division"` corresponds to `mo'o`.  `value` is a math-expression ID so
PA, lerfu, and eventually parenthesized ordinal expressions use the same
validated expression surface as mekso.

Some discourse connectives also assert a semantic relation between the
connected statements.  In that case the connected utterances remain in `items`,
and the additional relation is represented by formula IDs in
`connectionClaims`.  For example, `A .iri'abo B` asserts both `A` and `B`, and
also asserts a `rinka` predication whose x1 is the eventuality described by
`B` and whose x2 is the eventuality described by `A`.  Tense sentence
connectives use the same channel: `A .izu'abo B` asserts `A`, asserts `B`, and
adds a `leftOf(B-event, A-event)` connection claim.

Forethought tense connectives place the origin sentence first.  Thus `pugi A
gi B` asserts both `A` and `B`, and adds `before(B-event, A-event)`.  The
parallel pure-tense sumti and bridi-tail forms do not assert their branch
predications.  For `mi klama pugi le zarci gi le zdani`, the public content is
the asserted `before` relation; the two `klama` branch predications are present
only as `mode = "inert"` event descriptions whose eventualities fill the
relation arguments.  Logical+tensed sumti connectives such as `.ebabo` still
assert their logical branches and add the tense relation as an additional claim.

Grouped logical tense connectives use the same `connectionClaims` field at the
sequence level.  In `A .ije ba tu'e B .ija cabo C tu'u`, the outer sequence
keeps the logical `and` content and adds `after(group-event, A-event)`, where
the group-event is a reified eventuality whose `content` is the nested sequence.
The nested sequence keeps its own logical `or` content and its `at(C-event,
B-event)` connection claim.

`tu'e ... tu'u` text groups are represented as nested sequences even when the
group contains only one utterance.  This gives discourse pro-sumti such as
`di'e` a stable utterance target for the group while keeping the grouped
content itself sequence-shaped:

```json
{
  "type": "utterance",
  "force": "parenthetical",
  "content": "sequence:1009"
}
```

### eventuality referents

An eventuality is the event, state, process, activity, achievement, or locution
that a predication, utterance, or abstraction is about.  Eventualities are not
a separate public object type; they are `type:"referent"` objects whose `sort`
is `eventuality` or one of its hierarchical subsorts.

```json
{
  "type": "referent",
  "sort": "eventuality",
  "category": "constant",
  "actuality": { "kind": "actual" },
  "tenseModal": "parameter:1010",
  "timePath": [
    {
      "relation": "before",
      "anchor": {
        "kind": "object",
        "value": "eventuality:3"
      },
      "introducedBy": "pu"
    },
    {
      "relation": "after",
      "anchor": { "kind": "previous" },
      "introducedBy": "ba",
      "distance": "medium"
    }
  ],
  "timeInterval": {
    "extent": "whole",
    "anchor": "entity:1011"
  },
  "timeSpan": {
    "start": {
      "relation": "before",
      "anchor": "eventuality:3",
      "introducedBy": "pu",
      "distance": "medium"
    },
    "end": {
      "relation": "after",
      "anchor": "eventuality:3",
      "introducedBy": "ba",
      "distance": "long"
    },
    "introducedBy": "bi'o"
  },
  "aspect": {
    "contour": "completive",
    "anchor": "entity:1011"
  },
  "recurrence": [
    {
      "kind": "ordinalOccurrence",
      "introducedBy": "re'u",
      "quantity": "quantity:1012",
      "interval": "entity:1011"
    },
    {
      "kind": "occurrenceCount",
      "introducedBy": "roi",
      "value": { "integer": 1 }
    }
  ],
  "space": {
    "relation": "leftOf",
    "anchor": "entity:1013",
    "magnitude": {
      "value": "entity:1014",
      "introducedBy": "la'u"
    }
  },
  "spacePath": [
    {
      "relation": "inFrontOf",
      "anchor": {
        "kind": "object",
        "value": "entity:4"
      },
      "introducedBy": "ca'u",
      "distance": "short"
    },
    {
      "relation": "below",
      "anchor": { "kind": "previous" },
      "introducedBy": "ni'a",
      "distance": "medium"
    }
  ],
  "spaceInterval": {
    "extent": "medium",
    "directions": ["north"],
    "dimensions": ["line"],
    "anchor": "entity:1011"
  },
  "spatialAspect": {
    "contour": "initiative",
    "anchor": "entity:1011"
  },
  "spatialRecurrence": [
    {
      "kind": "regular",
      "introducedBy": "di'i",
      "interval": "entity:1011"
    }
  ]
}
```

`actuality` covers the CAhA family when it is explicit:

- `actual`
- `capable`
- `potential`
- `demonstrated`

Omitted CAhA is not equivalent to `ca'a`.  CLL 10.19 says a bridi without
CAhA may describe actual events, capabilities, or potential events depending
on context.  Therefore an eventuality with no explicit CAhA omits
`actuality`.  `ca'a` emits `{"kind":"actual"}`, `ka'e` emits
`{"kind":"capable"}`, `nu'o` emits `{"kind":"potential"}`, and `pu'i` emits
`{"kind":"demonstrated"}`.  Other tense attributes still compose with CAhA,
so `pu ca'a` has both `actuality.kind = "actual"` and
`time.relation = "before"`, while `ba nu'o` has
`actuality.kind = "potential"` and `time.relation = "after"`.

Temporal and spatial attributes should use structured anchors rather than only
flat labels.  This is an amendment from v0: the prelude had explicit moment,
region, path, current-location, and reference-time helpers, and the JSON model
should preserve that shape without copying the Lean API.

For a single temporal direction, use `time`.  For multiple cumulative temporal
directions, use `timePath` and omit `time`.  The first path step normally
anchors to an object such as `eventuality:3`; later unanchored steps use
`{"kind":"previous"}` to show that they are interpreted relative to the
previous step of the imaginary journey.  For example, `puba` is a two-step path
`before` then `after`, while `bapu` is `after` then `before`.  This order is
semantic and must not be collapsed into a single before/after field.
Temporal distance markers `zi`, `za`, and `zu` on a temporal direction become
`distance = "short"`, `"medium"`, or `"long"` on the affected `time` relation
or `timePath` step.  For example, `pu zu` is a single `time` relation with
`relation = "before"` and `distance = "long"`, while `pu ba za` is a
two-step `timePath` where only the `ba` step carries `distance = "medium"`.
When a temporal distance marker is not attached to a preceding temporal
direction, it is itself a distance-only temporal relation: `zi`, `za`, and
`zu` emit `time.relation = "near"`, `"mediumDistance"`, and `"far"` when they
stand alone.  In a compound such as `zi pu`, the output is a two-step
`timePath`: `near` anchored to speech time, then `before` anchored to the
previous step.  This preserves CLL 10.4's "near/far in time, direction
unspecified" reading instead of dropping the distance marker.

`aspect` covers a single ZAhO event-contour marker.  When multiple contour
markers occur in the same tense, `aspects` is an ordered list and the scalar
`aspect` field is omitted.  Use semantically transparent English contour
labels: `pu'o` is `prospective`, `ca'o` is `continuative`, `ba'o` is
`retrospective`, `co'a` is `initiative`, `co'u` is `cessative`, `mo'u` is
`completive`, `za'o` is `superfective`, `co'i` is `achievative`, `de'a` is
`pausative`, and `di'a` is `resumptive`.  The `co'u`/`mo'u` schema names
deliberately use the standard event-contour terms `cessative` and
`completive`; do not spell them as the English paraphrases "ceasing" or
"completed".

Contradictory tense negation with `nai` is formula-level negation of the
positive tensed predication for temporal/spatial relations and modal
attachments.  For `mi punai klama le zarci`, the utterance content is
`operator = "not"` whose child is the `klama` atom with
`time.relation = "before"`.  The event-time relation itself does not carry a
`negation` field, because that would double-count the contradiction.  The same
shape applies to sumtcita and aspects: `ne'inai le kumfa` wraps the atom whose
event has `space.relation = "within"` anchored to the room, and `ca'onai`
wraps the atom whose event has `aspect.contour = "continuative"`.  `nai`
after a ROI/TAhE interval property is different: it negates only that
recurrence property and is recorded on the recurrence entry, not as bridi
negation.

Scalar tense negation with `NAhE` asserts the predication with a modified
event relation or aspect.  `time`, `timePath[]`, `space`, `spacePath[]`,
`aspect`, and `spatialAspect` may carry `scalarNegation`:

```json
{
  "time": {
    "relation": "before",
    "anchor": "eventuality:3",
    "scalarNegation": {
      "kind": "otherThan",
      "introducedBy": "na'e"
    }
  },
  "space": {
    "relation": "within",
    "anchor": "entity:1011",
    "scalarNegation": {
      "kind": "opposite",
      "introducedBy": "to'e"
    }
  },
  "aspect": {
    "contour": "continuative",
    "anchor": "entity:1015",
    "scalarNegation": {
      "kind": "otherThan",
      "introducedBy": "na'e"
    }
  }
}
```

Spatial movement with `mo'i` is not the same as static location.  A spatial
relation introduced under `mo'i` carries `motion`:

```json
{
  "space": {
    "relation": "rightOf",
    "anchor": "entity:4",
    "motion": {
      "kind": "toward",
      "introducedBy": "mo'i"
    }
  }
}
```

When a compound spatial tense mixes static and moving steps, only the
`mo'i`-scoped step carries `motion`.  For example,
`zu'avu mo'i ri'uvi` produces a `spacePath` whose `leftOf`/`long` step is
static and whose `rightOf`/`short` step has `motion.kind = "toward"`.

`recurrence` is an ordered list of recurrence and interval-property markers
that modify the eventuality.  Known recurrence kinds include
`ordinalOccurrence` for `re'u`, `occurrenceCount` for `roi`, `regular` for
`di'i`, `typically` for `na'o`, `continuously` for `ru'i`, and `habitually`
for `ta'e`.  `recurrence` is a summary projection for consumers that only need
the recurrence layer.

`intervalModifiers` is the canonical ordered stack for interleaved ROI/TAhE
recurrences and ZAhO contours.  It is a heterogeneous list whose entries are
serialized as tagged wrappers such as:

```json
{
  "intervalModifiers": [
    {
      "kind": "recurrence",
      "value": {
        "kind": "occurrenceCount",
        "introducedBy": "roi",
        "quantity": "quantity:1012"
      }
    },
    {
      "kind": "aspect",
      "value": {
        "contour": "cessative"
      }
    }
  ]
}
```

The order is semantic and follows surface order, outermost first: CLL 10.10
contrasts `mi pare'u paroi klama le zarci` with
`mi paroi pare'u klama le zarci`, so the JSON must preserve whether
`ordinalOccurrence` wraps `occurrenceCount` or vice versa.  When a temporal
stack contains only recurrences, `recurrence` and `intervalModifiers` agree
except for the wrapper shape.  When it mixes recurrences and aspects,
`intervalModifiers` is authoritative for cross-operator order.

ROI counts reference full `quantity` objects.  The quantity uses
`scale = "frequency"` and preserves the PA form: `su'o roi` is
`form = "atLeast"`, `value.text = "su'o"`; `ro roi` is `form = "all"`,
`value.text = "ro"`.  Do not encode ROI counts as untyped recurrence strings.

`nai` after a **TAhE/ROI** interval property negates that recurrence property,
not the whole predication. (A ZAhO contour `-nai`, e.g. `ca'onai`, is *not*
recorded here — it is a bridi-level contradictory `not`-formula over the atom,
per the tense-negation prose above and CLL 10.18; see review-pass amendment 19.)
The recurrence entry carries `negation` so the source value
remains visible: `ru'inai` is a `continuously` recurrence with
`negation.introducedBy = "nai"`, and `reroinai` is an
`occurrenceCount` recurrence with a frequency `quantity` plus the same
negation field.

Non-logical connections between recurrence properties are recorded on the
following recurrence entry.  For `reroi pi'u xaroi`, the second
`occurrenceCount` entry carries
`connection = {"kind":"product","introducedBy":"pi'u"}` so the two counts are
understood as a cross-product of subevents rather than as unrelated flat
counts.

`timeInterval` records ZEhA temporal interval extents such as `short`,
`medium`, `long`, and `whole`.  `timeSpan` records BIhI/BIhO-bounded temporal
spans with explicit `start` and `end` endpoints, as in `puza bi'o bazu`: the
start is a medium-distance `before` endpoint relative to the anchor and the end
is a long-distance `after` endpoint relative to the same anchor.  `spaceInterval`
records VEhA/VIhA/FAhA spatial interval information with `extent`, `directions`,
and `dimensions`.  Dimensions use CLL-derived English labels: `line`, `area`,
`volume`, and `spaceTime`.  Directions use English compass or spatial relation
labels such as `north`.

When `fe'e` prefixes an interval property or ZAhO contour, the property is
spatial rather than temporal.  Such properties go in `spatialRecurrence`,
`spatialAspect`, `spatialAspects`, and the canonical
`spatialIntervalModifiers` stack, leaving `recurrence`, `aspect`, `aspects`,
and `intervalModifiers` for non-`fe'e` event/time interpretations.  For example,
`vi'i fe'e di'i` yields
`spaceInterval.dimensions = ["line"]` plus a `spatialRecurrence` entry with
`kind = "regular"`, while `fe'e co'a` yields
`spatialAspect.contour = "initiative"`.

`tenseModal` is an open slot for `cu'e` when the question asks for an
unspecified tense or modal construct.  The referenced parameter has
`sort = "tenseModal"` and `role = "tenseQuestion"`.  This is different from
`ca ma` or `vi ma`: those are ordinary `ma` parameters used as the anchor of a
known time or space relation, while `cu'e` leaves the relation, aspect,
actuality, or modal construct itself to be supplied by the answer.

Tense/modal fragments used as answers, such as `va`, `puzu`, `vi le lunra`,
`pu'o`, or `seka'a le briju`, are emitted as mentioned eventuality content.
The event carries the corresponding `space`, `time`, `aspect`, or
`modalArguments` fields directly.  This keeps answer fragments semantic without
inventing a full asserted predication.

Tense sumtcita anchor these same event attributes to the following sumti
instead of to the default speaker-relative deictic ground.  In
`mi klama le zarci ca le nu do klama le zdani`, the main predication's
eventuality has `time.relation = "at"` and `time.anchor` pointing to the
`le nu ...` referent, not to `eventuality:3`.  Likewise,
`ba'o le nu ...` records `aspect.anchor`, `reroi le ca djedi` records
`recurrence[].interval`, and `ze'u le ca dunra` records `timeInterval.anchor`.
Bare moved tense terms with elided `ku`, such as `puku`, still use the deictic
default because there is no following anchor sumti.

Subordinate tenses inherit the containing predication's event as their temporal
context.  In `mi pu klama le ba'o zarci`, the restrictive `zarci` eventuality
records `aspect.anchor` pointing at the main `klama` eventuality.  In
`mi ca jinvi le du'u mi ba morsi`, the `morsi` eventuality records
`time.anchor` pointing at the main `jinvi` eventuality.  If the subordinate
tense is itself compound, the first `timePath` step uses an object anchor for
the containing eventuality and subsequent unanchored steps use `previous`.
`nau` overrides this inherited temporal context for the current event only:
the event records `time.relation = "at"` and
`time.anchor = "eventuality:3"`.  It does not clear sticky tense state.

Story time is a contextual convention, not a syntactic feature.  The default
context-free lowering keeps ordinary sticky-tense behavior from CLL 10.13.  If
the caller explicitly supplies story-time context, asserted tenseless story
events are lowered as `time.relation = "after"` anchored to the previous story
event, explicit non-sticky temporal tenses are interpreted relative to the
current story event without advancing it, and explicit sticky temporal tenses
both use that current story event as their anchor and become the new story
anchor.  This uses the existing `time`/`timePath` fields; no separate
"story-time" object is required.

Tense constructs that are moved into term position with `ku`, such as
`puku mi klama le zarci`, are still event anchors.  They must not appear as
`modalArguments` with a fabricated `pu` relation and elided modal sumti.  CLL
10.1 says those forms differ from selbri-adjacent `pu` only in emphasis, so
the asserted predication's eventuality should carry the same
`time.relation = "before"` anchor.

Spatial distance tags such as `vi`, `va`, and `vu` on a selbri attach a
spatial anchor to the eventuality of that predication.  For example, in
`le vi bloti`, the boat description's restrictive predication has an
eventuality with `space.relation = "distanceFrom"`,
`space.distance = "short"`, and `space.anchor = "entity:4"`, rather than
treating `vi` as an extra place or dropping it after parsing.

Spatial direction tags such as `ne'i`, `zu'a`, and `ri'u` likewise attach a
spatial anchor.  For a single direction, use `space`; for multiple cumulative
spatial directions, use `spacePath` and omit `space`.  `spacePath` uses the
same step shape as `timePath`: the first step normally anchors to
`entity:4` or to a following tagged sumti, and later unanchored steps use
`{"kind":"previous"}`.  VA distance markers after a FAhA direction become
`distance = "short"`, `"medium"`, or `"long"` on that direction step.  VA by
itself uses the direction-neutral relation `distanceFrom` with the same
`distance` values.

Exact magnitudes from governed termsets use `magnitude` on the affected
`time`, `timePath[]`, `space`, or `spacePath[]` relation.  In
`sanli zu'a nu'i la .djordj. la'u lo mitre be li mu`, the `zu'a` spatial
relation has `anchor` pointing at George and
`magnitude.value` pointing at the `lo mitre be li mu` referent.  The termset
terms are consumed by the event relation and do not also fill ordinary `sanli`
places or create a separate `la'u` modal argument.  This is distinct from
vague `distance`: `vu` records a long but inexact distance, while
`magnitude` records an exact or independently described amount supplied by a
sumti.

Sticky spatial tenses use the same `ki` mechanism as sticky temporal tenses.
In `ne'i ki le kevna ... .i ...`, later predications inherit
`space.relation = "within"` anchored to the cave until a bare `ki` reset or a
new sticky spatial tense changes the spatial setting.  Anchor relations and
path steps produced by the marked tense carry `sticky = true`; copied-forward
relations carry `inherited = true` as well.  Bare `ki` reset clears the
sticky context for following predications; because it has no tense or space
relation of its own, it does not fabricate an anchor/path object.  This is
distinct from a BAI modal argument; FAhA/VA tenses are event-location
attributes.

When a modal relation's place structure calls for an event or state but the
connected side is a grouped formula or discourse sequence, reify an
eventuality whose `content` points at that formula or sequence.  Do not pick
the first atomic child as a proxy for the whole group.  Atomic asserted
predications still use their own predication eventualities, and tanru formulas
use their tertau/eventuality as their primary event.

```json
{
  "eventuality:22": {
    "type": "referent",
    "sort": "eventuality",
    "category": "constant",
    "actuality": { "kind": "actual" },
    "content": "formula:1016"
  },
  "formula:1016": {
    "type": "formula",
    "operator": "or",
    "connector": { "source": "a du'i bo", "locus": "sumti" },
    "children": [
      "formula:1017",
      "formula:1018",
      "formula:1019"
    ]
  },
  "predication:1020": {
    "type": "predication",
    "relation": "rinka",
    "introducedBy": "se ri'a",
    "arguments": {
      "x1": { "kind": "filled", "value": "eventuality:18" },
      "x2": { "kind": "filled", "value": "eventuality:22" },
      "x3": { "kind": "elided", "value": "entity:1021", "introducedBy": "zo'e" }
    }
  }
}
```

### referent

A referent is an entity-like value that can fill a place, be described, be
named, be quantified over, or be used as a discourse participant.

```json
{
  "type": "referent",
  "category": "constant",
  "sort": "entity",
  "descriptor": {
    "kind": "described",
    "word": "le",
    "speaker": "entity:1",
    "body": "formula:1022"
  }
}
```

`category` values:

- `constant`
- `variable`
- `indexical`
- `composite`

Indexical referents include discourse participants and deictic demonstratives.
`mi` and `do` resolve to the speaker and addressee; `ti`, `ta`, and `tu`
should be emitted as proximal, medial, and distal demonstrative indexicals
rather than as opaque pro-sumti constants.

`sort` values should be English semantic categories:

- `entity`
- `mass`
- `set`
- `sequence`
- `eventuality`
- `predication`
- `truthValue`
- `proposition`
- `concept`
- `amount`
- `quantity`
- `number`
- `scale`
- `text`
- `sign`
- `relation`
- `place`
- `connective`
- `tenseModal`
- `mathOperator`
- `argumentBundle`

Descriptor examples:

```json
{
  "kind": "veridicalDescription",
  "word": "lo",
  "body": "formula:1023"
}
```

Descriptor bodies are veridical by default and omit the field in that case.
When the body is explicitly characterizing rather than veridical, the
descriptor carries `"veridical": false`. The schema deliberately omits
`"veridical": true`.

```json
{
  "kind": "speakerDescription",
  "word": "le",
  "speaker": "entity:1",
  "body": "formula:1022"
}
```

Relative clauses that occur inside the description, before `ku` or its elided
equivalent closes the sumti, appear on the descriptor.  This keeps restrictions
that participate in selecting the described referent separate from restrictions
on a later occurrence of that referent:

```json
{
  "kind": "veridicalDescription",
  "word": "lo",
  "speaker": "entity:1",
  "body": "formula:1024",
  "relativeClauses": [
    {
      "kind": "incidental",
      "body": "formula:1025"
    }
  ]
}
```

A referent may carry context-local assigned names introduced with `goi`.  This
is not the same as making the referent a named-description referent: in
`le ninmu goi la .sam.`, CLL says that "Sam" is a name chosen for the current
context and does not imply that the woman's ordinary name is Sam.  Preserve that
with `assignedNames` on the described referent:

```json
{
  "type": "referent",
  "category": "constant",
  "sort": "entity",
  "descriptor": {
    "kind": "speakerDescription",
    "word": "le",
    "speaker": "entity:1",
    "body": "formula:1026"
  },
  "assignedNames": [
    {
      "name": "sam",
      "word": "la",
      "introducedBy": "goi",
      "source": { "text": "goi la .sam", "construct": "assigned-name" }
    }
  ]
}
```

Assignable pro-sumti handles use the same field.  In `le zarci goi ko'a`, the
store referent is still the `le zarci` description, but it carries:

```json
{
  "assignedNames": [
    {
      "name": "ko'a",
      "word": "ko'a",
      "introducedBy": "goi",
      "source": { "text": "goi ko'a", "construct": "assigned-name" }
    }
  ]
}
```

Mass and set gadri choose the referent sort directly.  For example, `loi
nu'a su'i nabmi` is a mass description:

```json
{
  "type": "referent",
  "category": "constant",
  "sort": "mass",
  "descriptor": {
    "kind": "veridicalMassDescription",
    "word": "loi",
    "body": "formula:1027"
  }
}
```

Number sumti introduced by `li` are number referents whose descriptor points at
a `quantity` object.  Simple numeric values can be represented directly:

```json
{
  "type": "referent",
  "category": "constant",
  "sort": "number",
  "descriptor": {
    "kind": "number",
    "word": "li",
    "quantity": "quantity:1028",
    "name": "vo"
  }
}
```

For non-trivial mekso, the quantity value points at a `mathExpression` object
rather than collapsing to opaque text:

```json
{
  "type": "referent",
  "category": "constant",
  "sort": "number",
  "descriptor": {
    "kind": "number",
    "word": "li",
    "quantity": "quantity:1029",
    "name": "re su'i re"
  }
}
```

```json
{
  "type": "quantity",
  "form": "exact",
  "value": {
    "mathExpression": "math:1030"
  },
  "scale": "count"
}
```

When a `li` sumti contains a lerfu string used as a mathematical variable, the
variable is interned by its lerfu-string name within the discourse.  Repeated
occurrences of `li ny.` therefore point at the same number referent, matching
CLL 17.11's treatment of lerfu strings as mathematical variables.  This is not
arithmetic normalization: `li vo` and `li re su'i re` remain distinct unless an
explicit bridi, connective, or later math solver relates them.

When a parenthesized mekso used as a sumti quantifier has a logical operand
connective, the connective has formula scope over the resulting quantified
claims.  For CLL 14.149/14.150, `vei ci .a vo prenu cu klama le zarci` is not a
single cardinality formula with an opaque quantity; it is an `or` formula with
`connector.locus = "mekso-operand"` whose children are the two cardinality
formulas using quantities 3 and 4.  Shared surrounding semantic material such as
`le zarci` remains shared by id.

`me'o` is different: it refers to the written/expression sign, not to the
numeric value.  Its sumti therefore emits a `sign` object:

```json
{
  "type": "sign",
  "kind": "mathExpression",
  "text": "re su'i re",
  "denotes": "math:1030"
}
```

```json
{
  "kind": "name",
  "word": "la",
  "name": "djan"
}
```

Sumti qualifiers such as `la'e` create a new referent derived from an operand
referent.  The descriptor `operand` field points at the inner sumti's referent;
it is a semantic reference, not a source substring.  For CLL 6.10's `la'e`,
the result is the thing referred to by a sign or symbol:

```json
{
  "type": "referent",
  "category": "constant",
  "sort": "entity",
  "descriptor": {
    "kind": "referentOfSymbol",
    "word": "la'e",
    "speaker": "entity:1",
    "operand": "entity:1031"
  }
}
```

Other LAhE-family qualifiers use the same operand shape, with descriptor kinds
such as `symbolForReferent`, `memberOf`, `setFrom`, `massFrom`, and
`sequenceFrom` where the source semantics are clear.

Metalinguistic pro-sumti such as `di'u` are referents to discourse items, not
opaque pro-sumti constants.  When reference analysis resolves them, the
referent carries `descriptor.kind = "utteranceReference"` and a top-level
`target` pointing at the utterance or sequence:

```json
{
  "type": "referent",
  "category": "constant",
  "sort": "sign",
  "descriptor": {
    "kind": "utteranceReference",
    "word": "di'u",
    "speaker": "entity:1"
  },
  "target": "utterance:1004"
}
```

The same shape covers the whole di'u-series.  `dei` targets the current
utterance object; `di'u` targets the previous discourse item when one is
available; `di'e` and the farther future/past forms may omit `target` and carry
a diagnostic when the referenced discourse item is outside the input.  `do'i`
is intentionally unspecified and therefore has no `target` and no unresolved
warning:

```json
{
  "type": "referent",
  "category": "constant",
  "sort": "sign",
  "descriptor": {
    "kind": "utteranceReference",
    "word": "do'i",
    "speaker": "entity:1"
  }
}
```

NAhE+BO sumti qualifiers such as `na'ebo le gerku` also create a qualified
referent rather than modifying the inner referent in place.  They reuse
`descriptor.operand`, point at a first-class scale referent with
`descriptor.scale`, and use semantic descriptor kinds:

- `otherThan` for `na'e bo`
- `oppositeOf` for `to'e bo`
- `neutralOf` for `no'e bo`
- `affirmedAs` for `je'a bo`

The scale referent is sorted as `scale`.  If the surface supplies an explicit
scale definition, such as with `ci'u`, the scale descriptor's `operand` points
at that definition.  If no scale definition is overt, the scale referent is
opaque: consumers know that the scalar operator is scale-relative, but not what
the contextual scale is.

`descriptor.definiteness` records which point on the scale is selected:

- `indefiniteAlternative` for `na'e bo`
- `uniqueExtreme` for `to'e bo`
- `neutralPoint` for `no'e bo`
- `affirmedPoint` for `je'a bo`

```json
{
  "type": "referent",
  "category": "constant",
  "sort": "entity",
  "descriptor": {
    "kind": "otherThan",
    "word": "na'e bo",
    "speaker": "entity:1",
    "scale": "scale:1032",
    "definiteness": "indefiniteAlternative",
    "operand": "entity:1033"
  }
}
```

```json
{
  "type": "referent",
  "category": "constant",
  "sort": "scale",
  "descriptor": {
    "kind": "scale",
    "word": "implicit scalar scale",
    "speaker": "entity:1",
    "name": "na'e bo"
  }
}
```

Sumti-based descriptions also use `operand`.  In `le re do`, the description is
not restricted by a selbri body; it describes a speaker-selected subset of the
referents of `do`.  The inner quantifier remains descriptor `quantity`, while
`operand` points at the embedded sumti's semantic object:

```json
{
  "type": "referent",
  "category": "constant",
  "sort": "entity",
  "descriptor": {
    "kind": "speakerDescription",
    "word": "le",
    "speaker": "entity:1",
    "quantity": "quantity:1034",
    "operand": "entity:2"
  }
}
```

In `le ti bloti`, the same `operand` field records the associated sumti
described in CLL 7.3: the referent is a speaker-selected boat whose description
is associated with the proximal demonstrative `ti`, not a direct translation of
English "this boat".

When a sumti appears between a descriptor and a following selbri, CLL 8.7 treats
it as an implicit `pe` relative phrase.  Keep `operand` as the structural pointer
to the possessor sumti, but also emit an `associatedWith` descriptor-relative
clause so the weak-association semantics are explicit:

```json
{
  "kind": "speakerDescription",
  "word": "le",
  "speaker": "entity:1",
  "body": "formula:1035",
  "relativeClauses": [
    {
      "kind": "restrictive",
      "body": "formula:1036"
    }
  ],
  "operand": "entity:1"
}
```

If a relative clause immediately follows the possessor sumti, as in
`le mi noi sipna vau karce`, attach that relative clause to the x2 argument of
the `associatedWith` predication, not to the described car.

Standalone sumti mentions can carry relative clauses directly on the referent
when the relative clause is part of the mentioned sumti rather than an
argument occurrence.  For `ti noi bloti`, the utterance content is the `ti`
referent, and that referent has an incidental relative clause:

```json
{
  "type": "referent",
  "category": "indexical",
  "sort": "entity",
  "indexical": "proximalDemonstrative",
  "relativeClauses": [
    {
      "kind": "incidental",
      "body": "formula:1037"
    }
  ]
}
```

This does not replace occurrence-scoped relative clauses on argument fillers:
those remain on `ArgumentValue` when a sumti occurrence fills a predication
place, so global indexicals such as `do` are not mutated by ordinary argument
uses.

Imperative `ko` is also occurrence-scoped.  It resolves to the addressee
referent, like `do`, and the host utterance has `force:"command"`, but the
specific argument occurrence that contained `ko` carries `commandTarget`:

```json
{
  "kind": "filled",
  "value": "entity:2",
  "commandTarget": { "introducedBy": "ko" }
}
```

This makes the commanded participant's place explicit without mutating the
global addressee referent.

Masses, sets, and non-logical composites are referents with structured
composition:

```json
{
  "type": "referent",
  "category": "composite",
  "sort": "mass",
  "composition": {
    "operator": "massOf",
    "members": ["entity:1038"],
    "excludedMembers": [],
    "collective": true
  }
}
```

```json
{
  "type": "referent",
  "category": "composite",
  "sort": "set",
  "composition": {
    "operator": "setOf",
    "members": ["entity:1039"]
  }
}
```

Logical sumti connectives with right-branch `nai` keep the positive and
excluded sides explicit.  For example, `mi .e nai do` has the speaker as a
member and the addressee as an excluded member:

```json
{
  "type": "referent",
  "category": "composite",
  "sort": "entity",
  "composition": {
    "operator": "joint",
    "members": ["entity:1"],
    "excludedMembers": ["entity:2"]
  }
}
```

Right-branch `nai` on a non-logical JOI connective is not the same thing.  CLL
14.15 treats it as scalar negation of the connection itself: neither side is
negated, but the named connection is marked as inapplicable.  For `mi jo'u nai
do`, keep both members and set `scalarNegated`:

```json
{
  "type": "referent",
  "category": "composite",
  "sort": "entity",
  "composition": {
    "operator": "joint",
    "members": ["entity:1", "entity:2"],
    "scalarNegated": true
  }
}
```

When the connective itself is questioned inside a sumti connection, use
`operator = "connectiveQuestion"` and point `operatorParameter` at a
`parameter` with `sort = "connective"` and `role = "connectiveQuestion"`:

```json
{
  "type": "referent",
  "category": "composite",
  "sort": "entity",
  "composition": {
    "operator": "connectiveQuestion",
    "operatorParameter": "parameter:1040",
    "members": ["entity:1041", "entity:1042"]
  }
}
```

BIhI interval connectives are also compositions, but their `nai` is an
interval complement rather than right-branch exclusion or JOI scalar negation.
When explicit GAhO endpoints occur, preserve their inclusivity on the
composition:

```json
{
  "type": "referent",
  "category": "composite",
  "sort": "entity",
  "composition": {
    "operator": "unorderedInterval",
    "members": ["entity:1043", "entity:1044"],
    "endpointInclusion": {
      "left": "inclusive",
      "right": "exclusive"
    }
  }
}
```

For `bi'i nai` or `bi'o nai`, use `complement: true`:

```json
{
  "type": "referent",
  "category": "composite",
  "sort": "entity",
  "composition": {
    "operator": "unorderedInterval",
    "members": ["entity:1043", "entity:1044"],
    "complement": true
  }
}
```

Non-logical composition can also combine concepts rather than ordinary entity
referents.  This is used when a JOI-series connective appears inside a tanru
modifier: the connected modifier is not a truth-functional formula connective,
but a composite concept used as the second argument of the vague tanru
relation.

```json
{
  "type": "referent",
  "category": "composite",
  "sort": "concept",
  "composition": {
    "operator": "mass",
    "members": ["relation:1045", "relation:1046"],
    "collective": true
  }
}
```

Non-logical connective operators include at least:

- `joint`
- `mass`
- `set`
- `sequence`
- `respectively`
- `union`
- `intersection`
- `crossProduct`
- `unorderedInterval`
- `orderedInterval`
- `centeredInterval`

This is an amendment from the earlier object model, which mentioned
non-logical JOI-style combinations but did not give composite referents a
complete attribute shape.

### parameter

A parameter is an open semantic slot that appears inside some other object.
It is not a general "answer object".  Use a parameter only when the semantic
body actually contains a fillable position: `ce'u` in a property abstraction,
`ke'a` in a relative clause, direct `ma`, direct `mo`, direct `fi'a`, or a
missing connective/tense/operator requested by a question.

```json
{
  "type": "parameter",
  "sort": "entity",
  "role": "propertySlot",
  "introducedBy": "ce'u"
}
```

`propertySlot` parameters use the sort required by the property's domain.  A
typical `ka broda` property has an entity slot, but a property made from an
event abstraction such as the seltau of `nu sonci kei djica` has an
eventuality slot:

```json
{
  "type": "parameter",
  "sort": "eventuality",
  "role": "propertySlot",
  "introducedBy": "ce'u"
}
```

Use `question` objects for the interrogative act or indirect-question value.
The parameter is only the slot that the answer would fill.  A `xu` truth
question does not need a parameter, because the answer is the truth value of
the whole body.  `kau` does not by itself create a parameter either: if the
marked constituent is `ma kau`, the question focus can point at the `ma`
parameter; if it is `la .djan. kau`, the focus points at the `djan` referent.

`role` values include:

- `propertySlot`
- `relativeClauseHead`
- `argumentQuestion`
- `relationQuestion`
- `relationVariable`
- `placeQuestion`
- `connectiveQuestion`
- `tenseQuestion`
- `mathOperatorQuestion`
- `attitudeQuestion`

A relation-question parameter has `sort = "relation"` and is used directly by
a predication's `relationParameter` field.  It is not represented as a lexical
relation named `mo`, because the question asks which relation should connect
the supplied arguments.

A relation-variable parameter also has `sort = "relation"`, but
`role = "relationVariable"`.  It is used for bound selbri variables such as
`bu'a`, `bu'e`, and `bu'i`.  The predication uses the parameter through
`relationParameter`, and the quantifier formula binds that same parameter as
its `variable`:

```json
{
  "parameter:1047": {
    "type": "parameter",
    "sort": "relation",
    "role": "relationVariable",
    "introducedBy": "bu'a"
  },
  "predication:1048": {
    "type": "predication",
    "relationParameter": "parameter:1047",
    "arguments": {
      "x1": { "kind": "filled", "value": "entity:1049" },
      "x2": { "kind": "filled", "value": "entity:1050" }
    },
    "mode": "asserted"
  },
  "formula:1051": {
    "type": "formula",
    "operator": "exists",
    "variable": "parameter:1047",
    "body": "formula:1052"
  }
}
```

A place-question parameter has `sort = "place"` and is anchored by a
predication's `placeQuestions` field.  It is not an entity filler: `fi'a do`
does not ask who fills a place, but which place the known addressee fills.

A tense-question parameter has `sort = "tenseModal"` and is referenced from an
eventuality's `tenseModal` field.  It represents the open tense/modal
construct requested by `cu'e`, not an entity standing for a time or place.

A connective-question parameter has `sort = "connective"` and is referenced
from a formula connector's `parameter` field.  It represents the missing
truth-table connective requested by `je'i`.

A math-operator question parameter has `sort = "mathOperator"` and is
referenced from a `mathExpression` object's `operatorParameter` field.  It
represents the missing mekso operator requested by forms such as `na'u mo`;
this is distinct from direct bridi `mo`, which asks for a relation and uses
`relationParameter`.

Vocative questions such as `doi ma` still use an ordinary
`argumentQuestion` parameter, but the question is local to the vocative
utterance.  The body is a performative `vocativeTarget` predication whose
target place contains the parameter:

```json
{
  "type": "utterance",
  "force": "vocative",
  "content": "question:1053"
}
```

```json
{
  "type": "predication",
  "relation": "vocativeTarget",
  "arguments": {
    "x1": { "kind": "filled", "value": "parameter:1054" }
  },
  "mode": "performative"
}
```

### selbri-derived relation bodies

Some grammar productions consume a selbri as the content of another construct
rather than as the ordinary main bridi-tail.  These constructs still receive
the same semantic lowering as any other selbri: tanru, `co`, `bo`, `ke`, SE
conversion, JAI, NAhE, linkargs, NU/ME/NUhA-derived units, logical and
non-logical selbri connectives, elided `zo'e`, deleted `zi'o`, and dictionary
place structure must survive structurally.  Do not introduce a raw
syntax-shaped `selbriExpression` object, and do not collapse complex selbri to
text labels or compound relation strings.

The relevant wrappers are:

- `(LA | LE) ... selbri` and `quantifier selbri`, which build descriptions,
  names, and bare quantified sumti.
- Bare-selbri vocatives, which CLL 6.11 treats as implicit `le` descriptions
  of the addressee.
- `FIhO selbri FEhU`, which builds an ad-hoc modal body.
- `NAhU selbri TEhU`, which builds a selbri-derived mekso operator.
- `NIhE selbri TEhU`, which builds a selbri-derived mekso operand.
- `SEI [terms [CU]] selbri SEhU`, which is a metalinguistic bridi with a
  restricted surface syntax but ordinary predication semantics.

The wrapper determines how the visible x1 or output value is supplied:

- `le broda` uses the structural `skicu(speaker, referent, audience,
  ka ce'u broda)` characterization; `broda` is inside the `ka` body, not
  predicated directly of the described referent.
- `la broda` uses the analogous `cmene(sign, referent, namer)` clause.  The
  sign preserves the name text and may point at the lowered selbri relation
  body as its meaning; the named referent is not asserted to satisfy `broda`.
- A bare-selbri vocative uses the same description body as `le <selbri>`, with
  the resulting referent serving as the vocative audience.
- `fi'o <selbri> <sumti>` fills the tagged selbri's visible x1 with the modal
  sumti and attaches that subordinate body as the host predication's modal
  argument.
- `na'u <selbri>` builds a typed math operator whose result place is the
  selbri's visible x1 and whose later unfilled places are operands.
- `ni'e <selbri>` builds a typed math operand from the selbri's output value.
  When the selbri explicitly yields an amount, as in `ni'e ni clani`, the
  operand denotes that quantity output; otherwise the result slot and body must
  still be represented structurally rather than as opaque text.
- `sei ... <selbri>` builds a nested metalinguistic utterance or aside whose
  content is the lowered bridi body.

When a wrapper needs a relation-valued object rather than an immediate
predication body, use a relation-sorted referent with `relationKind = "selbri"`,
`parameters`, `arity`, and `body`.  This is the direct relation-output analogue
of `ka`, but it records that the relation came from a bare selbri wrapper rather
than from an explicit abstraction cmavo.

### predication

A predication applies a relation to arguments under an eventuality.

```json
{
  "type": "predication",
  "relation": "klama",
  "eventuality": "eventuality:18",
  "arguments": {
    "x1": { "kind": "filled", "value": "entity:1" },
    "x2": { "kind": "filled", "value": "entity:1055" },
    "x3": {
      "kind": "elided",
      "value": "entity:1056",
      "introducedBy": "zo'e"
    },
    "x4": {
      "kind": "deleted",
      "introducedBy": "zi'o"
    },
    "x5": {
      "kind": "elided",
      "value": "entity:1057",
      "introducedBy": "zo'e"
    }
  },
  "modalArguments": [
    {
      "relation": "zgana",
      "introducedBy": "ga'a",
      "arguments": {
        "x1": {
          "kind": "filled",
          "value": "entity:1"
        },
        "x2": {
          "kind": "elided",
          "value": "entity:1058",
          "introducedBy": "zo'e"
        },
        "x3": {
          "kind": "elided",
          "value": "entity:1059",
          "introducedBy": "zo'e"
        },
        "x4": {
          "kind": "elided",
          "value": "entity:1060",
          "introducedBy": "zo'e"
        }
      }
    }
  ],
  "reciprocity": [
    {
      "left": { "kind": "filled", "value": "entity:1" },
      "right": { "kind": "filled", "value": "entity:2" },
      "introducedBy": "soi"
    }
  ],
  "scalarNegation": {
    "kind": "otherThan",
    "introducedBy": "na'e"
  },
  "mode": "asserted"
}
```

Usually `relation` is a lexical relation name such as `klama`.  If the
relation itself is questioned, as with direct `mo`, use `relationParameter`
instead of `relation`:

```json
{
  "type": "predication",
  "relationParameter": "parameter:1061",
  "arguments": {
    "x1": { "kind": "filled", "value": "entity:2" }
  },
  "mode": "asserted"
}
```

The referenced parameter should have `sort = "relation"` and
`role = "relationQuestion"`.  The predication still contains the supplied
argument fillers, because an answer fills the relation slot while preserving
the visible places unless the answer is a full bridi whose arguments override
them.

If the place itself is questioned, as with direct `fi'a`, keep the ordinary
`arguments` map for known numbered places and add `placeQuestions` entries.
Each entry points at a place-sorted parameter, the known argument whose place is
being asked about, and the candidate places that remain open after ordinary
numbered assignments have been applied:

```json
{
  "type": "parameter",
  "sort": "place",
  "role": "placeQuestion",
  "introducedBy": "fi'a"
}
```

```json
{
  "type": "predication",
  "relation": "dunda",
  "arguments": {
    "x1": {
      "kind": "elided",
      "value": "entity:1056",
      "introducedBy": "zo'e"
    },
    "x2": { "kind": "filled", "value": "entity:1062" },
    "x3": {
      "kind": "elided",
      "value": "entity:1057",
      "introducedBy": "zo'e"
    }
  },
  "placeQuestions": [
    {
      "parameter": "parameter:1063",
      "argument": { "kind": "filled", "value": "entity:2" },
      "candidatePlaces": ["x1", "x3"]
    }
  ],
  "mode": "asserted"
}
```

The candidate list excludes x2 here because CLL 9.3's `fi'a do dunda fe le vi
rozgu` already fills x2 with the rose.  The normal argument entries remain
explicit so that omitted candidate places are still distinct from deletion.

`mode` values include:

- `asserted`
- `definitional`
- `restrictive`
- `incidental`
- `displayed`
- `inert`
- `performative`

The identity cmavo `du` emits `relation = "identity"` rather than the Lojban
word itself.  Its predication mode is `definitional`, because CLL 7.14 treats
it as an identifying sentence about attached sumti representing the same
referent, not as the ordinary sameness predicate `mintu`.

Argument values are structured so that omission and deletion cannot be
conflated:

- `filled.value` points at the semantic object supplied by an explicit sumti or
  parameter.
- `elided.value` points at a fresh explicit referent for `zo'e`; `introducedBy`
  is normally `zo'e`, whether the `zo'e` was written or supplied by omission.
- `deleted` records a `zi'o` place deletion.  It has no `value`, creates no
  referent, and carries no existential import.

Most argument fillers are ordinary referents, parameters, eventualities,
abstractions, signs, math expressions, quantities, or displayed content.  A
`filled.value` may also point at a `formula` when the source relation takes a
proposition-like argument.  For example, `ni'i` is based on `nibli`, whose x1
and x2 are propositions; in `li ny. du li vo .ini'ibo li ny. du li re su'i re`,
the generated `nibli` predication fills x1 and x2 with the two identity formula
IDs rather than with event IDs.

Outer quantifiers do not live on `ArgumentValue`.  They introduce formula-level
restricted-variable scopes, even when the quantified source appears in a single
argument position.  For example, `re do cadzu le bisli` introduces a variable
selected from the addressee group with `quantity:1034`, and the walking
predication uses that variable as x1.  Likewise, `ci lo gerku cu bajra` keeps
the `lo gerku` description as a constant referent and quantifies a variable
restricted by membership in that description.  This keeps quantifier scope,
restriction, and re-quantification visible at the formula layer instead of
mutating an argument occurrence or the global referent it points at.

Inner quantifiers inside an explicit description appear on the descriptor
instead.  For `le ci gerku`, the descriptor carries the `ci` quantity:

```json
{
  "type": "referent",
  "category": "constant",
  "sort": "entity",
  "descriptor": {
    "kind": "description",
    "word": "le",
    "speaker": "entity:1",
    "body": "formula:1064",
    "quantity": "quantity:1065"
  }
}
```

When an explicit outer quantifier ranges over an explicit description, the
description remains a constant referent and the quantified variable is
restricted with `memberOf(variable, description)`.  Thus `ci lo gerku` retains
the `lo gerku` referent, and `so'o lo ci gerku` also keeps the inner `ci` as the
description's `quantity`.

Every dictionary-known place of a relation should have an `arguments` entry.
If a place is omitted, it is represented by an `elided` value with its own
`zo'e` referent.  If a place is explicitly deleted with `zi'o`, it is
represented by `deleted`.  Relations whose arity is unavailable should emit the
assigned places that are forced by syntax, plus a diagnostic that the full
place structure is unavailable; the graph must not invent unbounded places.

Tagged or modal sumti such as `ga'a mi` are not numbered places.  They appear
in `modalArguments`, where `relation` is the source relation for the tag
(`zgana` for `ga'a`, `klama` for `ka'a`, and so on), `introducedBy` preserves
the source marker, and `arguments` uses the same structured numbered place map
as predication arguments.  This keeps `be ga'a mi` attached to the linked
modifier predicate while tail `ga'a mi` attaches to the enclosing predication.
The deliberately vague modal `do'e` uses `relation = "unspecified-modal"` and
only records the visible modal place.
Reference-frame tags such as `ma'i vo'a` use the source relation `manri` in
`modalArguments`, with the resolved reference-frame sumti filling the visible
place.

`modalArguments[].component`, when present, identifies the component of a
composite argument that the modal applies to.  CLL 14.131/14.132 requires this:
`mi ce'e bau la .lojban. pe'e joi do ce'e bau la .gliban. casnu` has one
`casnu` predication whose x1 is the mass of `mi` and `do`, but the `bau` modal
for Lojban applies to the speaker component and the `bau` modal for English
applies to the addressee component.  Without `component`, the two language
modals would incorrectly describe the whole mass uniformly.

A modal entry has either a lexical modal `relation` plus structured
`arguments`, or a full modal `body` formula.  BAI-derived tags normally use the
lexical relation shape because their source relation is fixed by the marker.
`fi'o` uses the `body` shape because its source is an arbitrary selbri whose
tanru structure, linkargs, conversion, and other selbri-internal semantics must
remain visible.  This is true even when the source selbri is a single brivla:
`fi'o kanla` still contributes a modal predication body with every known
`kanla` place explicit, not a relation-string shorthand.

Ad-hoc modal tags with `fi'o` take a full selbri body.  The modal entry uses
`body` to point at a subordinate formula for that tagged selbri; the following
modal sumti fills the selbri's visible x1 after conversion, `be`/`bei` linkargs
fill the places they govern, and all other dictionary-known omitted places are
explicit `elided` fillers.  Thus `fi'o kanla le zunle` contains a modal body
with `kanla(x1 = le zunle, x2 = zo'e)`, while `fi'o se pilno le zunle kanla`
contains a modal body with raw `pilno` x2 filled by the tool and x1/x3 elided.
For tanru such as `fi'o melbi kanla`, the body uses the normal tanru schema
rather than a flat relation string such as `"melbi kanla"`.

Modal tags before a selbri, as in `mi bai tavla` and `mi fi'o kanla fe'u viska
do`, also appear in `modalArguments` on the affected predication.  Because
there is no explicit modal sumti, the tag body's visible modal place is an
`elided` filler, and all other dictionary-known places of the tag relation are
also explicit elisions.  This preserves the CLL 9.9 "modal selbri" reading
without inventing a compeller, tool, eye, or other participant.

When a BAI source relation links two events or propositions, the host
predication's eventuality fills the complementary source-relation place that is
not occupied by the tagged sumti after SE conversion.  Thus `ri'a X` fills
`rinka` x2 with the host event and x1 with `X`, while `se ri'a X` fills x1 with
the host event and x2 with `X`.

`jai` with a BAI modal conversion, such as `la .lojban. jai bau cusku fai mi`,
uses the same `modalArguments` shape.  The base predication remains the inner
source relation (`cusku`), `fai` supplies the original x1 when present, and the
raised x1 fills the visible place of the BAI source relation (`bangu` x1 for
`bau`).  No public binding or conversion object is needed, because the resolved
graph can state the resulting predicate and modal relation directly.
CLL 11.10's bare `jai` is vaguer: it raises something from an implicit
abstraction involving the surface x1.  Public JSON represents that implicit
abstraction with a normal `referent` descriptor
`{"kind":"abstractionAbout","word":"jai","operand":...}` and uses that
referent in the inner predicate's appropriate place.  This parallels explicit
`tu'a`, whose descriptor uses `word = "tu'a"`.  If the base relation's moved
place is already filled, as with `do jai se krinu ... fai le nu mi lebna ...`,
the base predicate keeps its ordinary place routing and an asserted constructed
relation `involves(x1 = moved abstraction/event, x2 = raised participant)` is
conjoined with it.  This preserves both the underlying relation
`krinu(reason, justified-event)` and the bare-`jai` claim that the raised
surface x1 participates in the abstraction moved to `fai`.  When `jai` has a
BAI marker, as in `le jai gau rinka ...`, the described referent fills the BAI
source relation in `modalArguments` (`gasnu` for `gau`), while the inner
predicate's old x1 remains filled by `fai` if present or is otherwise an
explicit `zo'e` elision.

Modal negation belongs to the modal relation, not to the host predication.
`BAI nai` adds `negation` with `kind = "contradictory"` to the
`modalArguments` entry.  `NAhE BAI` adds `scalarNegation` using the same scalar
negation object shape used for selbri negation.  In `mi nelci do mu'inai le nu
do nelci mi`, the speaker's liking is still asserted, while the `mukti` modal
relation is contradicted; in `banro na'emu'i ...`, the `mukti` modal relation
is marked as other-than, without inventing which alternate relation holds.

Indicators attached to a modal tag modify the modal relation itself, not the
host predication as a whole.  They appear as nested `modifiers` on the
`modalArguments` entry, using the same displayed-content modifier shape as
attitudinal modifiers.  This is needed for CLL 15.10 `go'i ji'una'iku`, where
`na'i` marks the `ji'u` presupposition/assumption as metalinguistically wrong:

```json
{
  "type": "predication",
  "relation": "go'i",
  "modalArguments": [
    {
      "relation": "ji'u",
      "introducedBy": "ji'u",
      "arguments": {
        "x1": {
          "kind": "elided",
          "value": "entity:1011",
          "introducedBy": "zo'e"
        }
      },
      "modifiers": [
        {
          "relation": "metalinguisticNegation",
          "family": "metalinguistic",
          "polarity": "positive",
          "assertionEffect": "metalinguisticallyVoided"
        }
      ]
    }
  ]
}
```

Sticky modals are represented by their semantic effect, not by a public binding
object.  When `ki` makes a BAI modal sticky, the resolved `modalArguments`
entry is repeated on following asserted predications until sticky state is
cancelled.  The repeated entry reuses the same semantic IDs in its place map,
because CLL 9.14 says the modal and its following sumti persist together.  Bare
`ki` before a selbri clears sticky modal context and produces no modal argument
of its own.

Logical connection between modal tags, as in `seka'a je teka'a le zdani`,
does not make one modal relation named after the whole tag string.  It lowers
to the corresponding logical formula over host predications: one branch has a
`se ka'a`/`klama` modal argument and another branch has a `te ka'a`/`klama`
modal argument.  The explicit modal sumti resolves once and is reused in the
visible place of each branch's modal relation.  This differs from a `ce'e`
termset such as `seka'a le zdani ce'e teka'a le zdani`, which CLL 9.15 treats
as one host event with both modal arguments.

When one modal scopes over a connected bridi-tail or a `tu'e` text group, the
same modal relation should be attached to every asserted predication in that
scope.  The repeated JSON entries reuse the same semantic IDs inside their
`arguments` maps, so a single elided modal operand remains one discourse
referent across all affected predications.  Restrictive predications inside
descriptions are not affected by the outer modal.

Reciprocal `soi` modifiers appear in `reciprocity` on the affected
predication.  Each entry records the two participant fillers whose interchange
is asserted to preserve the bridi truth.  Participants use the same structured
argument filler shape as predication places, but they must not be `deleted`.
When `soi` has only one explicit participant, the missing participant is the
immediately preceding sumti and should resolve to that sumti's existing
predication-place filler.  In `mi prami do soi vo'a`, for example, `left`
resolves to the current x1 (`entity:1`) and `right` resolves to the x2
host sumti (`entity:2`).  In `mi bajykla ti ta soi vo'e`, the x2
and x3 argument fillers are reused rather than constructing duplicate `ti` or
`ta` referents.

Scalar negation uses `scalarNegation` on the predication whose relation is
being modified.  It is not formula negation: `na'e cadzu` says that the true
relation is something other than walking, not that the walking predication is
false.  `kind` values include `otherThan` for `na'e`, `opposite` for `to'e`,
`neutral` for `no'e`, and `affirmed` for `je'a`; `introducedBy` preserves the
surface marker.

`scalarNegation.scale` points at the first-class scale referent used by the
scalar operator.  With `be ci'u ...`, the scale referent's descriptor uses
`word:"ci'u"` and `operand` points at the overt scale definition.  Without an
overt scale, the scale referent is opaque (`word:"implicit scalar scale"`).

```json
{
  "type": "predication",
  "relation": "xunre",
  "scalarNegation": {
    "kind": "otherThan",
    "introducedBy": "na'e",
    "scale": "scale:1032"
  }
}
```

```json
{
  "type": "referent",
  "category": "constant",
  "sort": "scale",
  "descriptor": {
    "kind": "scale",
    "word": "ci'u",
    "speaker": "entity:1",
    "name": "na'e",
    "operand": "relation:1066"
  }
}
```

`scalarNegation.argumentScope`, when present, lists the numbered places that
are syntactically inside the scalar-negated selbri unit.  This distinguishes
CLL 15.53 `na'e ke sutra cadzu ke'e lemi birka`, where the arm sumti is a
trailing bridi argument outside the scalar operator, from 15.54
`na'e ke sutra cadzu be lemi birka ke'e`, where the `be`-attached x2 is inside
the scalar-negated unit:

```json
{
  "scalarNegation": {
    "kind": "otherThan",
    "introducedBy": "na'e",
    "scale": "scale:1032",
    "argumentScope": ["x1", "x2"]
  }
}
```

Relative clauses on a sumti occurrence appear inside that occurrence's argument
filler:

```json
{
  "kind": "filled",
  "value": "entity:2",
  "relativeClauses": [
    {
      "kind": "incidental",
      "body": "formula:1067"
    }
  ]
}
```

This is intentionally occurrence-scoped.  `do noi barda` should not mutate the
global addressee referent; it should qualify that use of the addressee.  The
same shape also distinguishes `xamgu be do noi barda` from
`xamgu be do be'o noi barda`: before `be'o`, the clause appears on the linked
`do` filler, while after `be'o` it is inside the outer description and appears
on that referent's descriptor.

Relative phrases introduced by GOI-family cmavo lower to the same attachment
shape.  The optional `introducedBy` field preserves the surface relative-phrase
marker when the body was introduced by a shortcut phrase rather than a full
`poi`/`noi` bridi.  The body points at an ordinary formula whose predication
records the semantic relation:

- `pe` and `ne` use `relation = "associatedWith"`; `pe` is restrictive and
  `ne` is incidental.
- `po` uses `relation = "specificallyAssociatedWith"` and is restrictive.
- `po'e` uses `relation = "intrinsicallyPossessedBy"` and is restrictive.
- `po'u` and `no'u` use `relation = "identity"`; `po'u` is restrictive and
  `no'u` is incidental.  Unlike `du`, these identity predications use the
  relative-clause mode, not `mode = "definitional"`.
- Modal relative phrases such as `pe cu'u la .artr.` and `ne semau la
  .meiris.` keep the same `RelativeClause` attachment and `introducedBy =
  "pe"`/`"ne"` on the attachment, but the body predication uses the BAI source
  relation rather than vague `associatedWith` when the source place routing is
  known from CLL.  The predication's own `introducedBy` preserves the modal
  marker, e.g. `"cu'u"` or `"se mau"`.

For `le stizu pe mi cu blanu`, the main predication's x1 filler therefore
points at a restrictive clause introduced by `pe`, and that clause body is
`associatedWith(chair, speaker)`:

```json
{
  "kind": "filled",
  "value": "entity:1068",
  "relativeClauses": [
    {
      "kind": "restrictive",
      "body": "formula:1036",
      "introducedBy": "pe"
    }
  ]
}
```

For `la .apasonatas pe cu'u la .artr. cu se nelci mi`, the occurrence of the
Appassionata in `nelci` points at a restrictive relative phrase, while the body
recovers the CLL 9.10 source relation `cusku(Arthur, Appassionata, zo'e, zo'e)`:

```json
{
  "kind": "filled",
  "value": "entity:1069",
  "relativeClauses": [
    {
      "kind": "restrictive",
      "body": "formula:1070",
      "introducedBy": "pe"
    }
  ]
}
```

```json
{
  "type": "predication",
  "introducedBy": "cu'u",
  "relation": "cusku",
  "arguments": {
    "x1": { "kind": "filled", "value": "entity:1071" },
    "x2": { "kind": "filled", "value": "entity:1069" },
    "x3": { "kind": "elided", "value": "entity:1011", "introducedBy": "zo'e" },
    "x4": { "kind": "elided", "value": "entity:1015", "introducedBy": "zo'e" }
  },
  "mode": "restrictive"
}
```

Non-veridical relative clauses introduced by `voi` are restrictive attachments,
but their body is not the described predicate asserted of the head.  Instead,
the body is a `describedAs` predication whose x1 is the speaker, x2 is the head
referent, and x3 is a property abstraction for the `voi` bridi.  The attachment
preserves the surface marker and marks the non-veridicality explicitly:

```json
{
  "kind": "filled",
  "value": "entity:1072",
  "relativeClauses": [
    {
      "kind": "restrictive",
      "body": "formula:1073",
      "introducedBy": "voi",
      "veridical": false
    }
  ]
}
```

### formula

A formula is the truth-bearing layer.  Atomic formulas point at predications;
compound formulas express logical operators or quantification.

Atomic:

```json
{
  "type": "formula",
  "operator": "atom",
  "predication": "predication:1048"
}
```

Bridi-level `ja'a` affirmation wraps its child formula with
`operator:"affirmed"`.  This is a formula-layer marker for explicit assertion;
it is distinct from scalar `je'a`, which remains `scalarNegation.kind:"affirmed"`
on a predication or related scalar carrier.

```json
{
  "type": "formula",
  "operator": "affirmed",
  "children": ["formula:1052"],
  "source": { "text": "ja'a", "construct": "bridi-affirmation" }
}
```

Connective:

```json
{
  "type": "formula",
  "operator": "and",
  "children": ["formula:1074", "formula:1075"],
  "connector": {
    "source": "je",
    "locus": "selbri",
    "truthTable": "TFFF"
  }
}
```

Scoped tense or modal:

```json
{
  "type": "formula",
  "operator": "scoped",
  "eventuality": "eventuality:18",
  "children": ["formula:1076"]
}
```

Quantifier:

```json
{
  "type": "formula",
  "operator": "exists",
  "variable": "entity:1077",
  "restriction": "formula:1078",
  "body": "formula:1079",
  "quantity": "quantity:1080"
}
```

A restricted universal carries its non-classical import explicitly:

```json
{
  "type": "formula",
  "operator": "forall",
  "variable": "entity:1081",
  "restriction": "formula:1082",
  "domainImport": "projective",
  "body": "formula:1083",
  "quantity": "quantity:1084"
}
```

The normative restriction/import semantics are operator-specific:

- `exists` and `pluralExists`: `restriction` is conjoined with `body` inside
  the existential claim. Any nonempty-domain consequence is at issue and
  classically entailed; `domainImport` is absent.
- `cardinality`: `restriction` limits the counted witnesses and its existence
  consequences are exactly those classically entailed by the quantity; there
  is no projective import and `domainImport` is absent.
- `forall` and `pluralForall`: `restriction` is the restricted domain (the
  implication antecedent for the at-issue universal). When it is present,
  that domain is additionally required to be nonempty **projectively**: the
  commitment survives `not`. Such a node must have
  `domainImport = "projective"`. With no `restriction`, it has no marker and
  makes no restricted-domain existence claim.
- `none`: the reading is classical no-witness / negated existential over the
  restriction and body. It does not import a witness; `domainImport` is
  absent.

`domainImport` is a closed enum whose only current value is `"projective"`.
It appears if and only if the formula operator is `forall` or
`pluralForall` **and** `restriction` is present. Consumers must interpret it
as a projective commitment that some value satisfies `restriction`, not as a
second child formula or an at-issue conjunct. The field is omitted everywhere
else, including restricted existential/cardinality nodes, `none`, constants,
and Skolem co-variation of `zo'e`.

Quantificational pro-sumti (`da`, `de`, `di`) use formula-level quantifier
wrappers when their quantifier has semantic scope over the bridi.  In
`ro da poi prenu cu prami pa de poi finpe`, the root content is a `forall`
formula for `ro da`; its body is a `cardinality` formula for `pa de`; the
`poi` clauses provide the respective `restriction` formulas.  This preserves
the CLL distinction between "every person loves one fish each" and a single
wide-scope fish.

A bare da-series variable has implicit existential force.  It uses the same
quantified formula shape with `operator = "exists"` and a `variable`, but no
`quantity` object, because there is no overt numeric quantifier to represent.
Repeated mentions of the same variable within the scope point at the same
referent and do not introduce additional wrapper formulas.

If an overt quantifier is applied to an already-bound da-series variable, the
new occurrence binds a fresh selected variable and records the source witness
set explicitly.  For example, in `ci da poi prenu cu se ralju pa da`, the
`pa da` formula is a cardinality quantifier over a new `entity:1081`;
`sourceVariable` and `selectionSource.variable` both point at the earlier
`entity:1082`, and inherited restrictions such as `poi prenu` are copied
as restrictions on the selected variable:

```json
{
  "formula:1083": {
    "type": "formula",
    "operator": "cardinality",
    "variable": "entity:1081",
    "sourceVariable": "entity:1082",
    "selectionSource": {
      "kind": "witnessSet",
      "variable": "entity:1082"
    },
    "restriction": "formula:1084",
    "body": "formula:1079",
    "quantity": "quantity:1085"
  }
}
```

Grouping termsets (`ce'e`, or `nu'i...nu'u` without a connective) equalize the
scope of their quantified terms.  They use one formula with
`operator:"quantifierBundle"`, ordered `bindings`, one shared `body`, and
`coequalScope:true`.  Each binding carries the same fields an ordinary
quantified formula would have.

```json
{
  "formula:1086": {
    "type": "formula",
    "operator": "quantifierBundle",
    "bindings": [
      {
        "operator": "cardinality",
        "variable": "entity:1087",
        "restriction": "formula:1088",
        "quantity": "quantity:1065"
      },
      {
        "operator": "cardinality",
        "variable": "entity:1089",
        "restriction": "formula:1090",
        "quantity": "quantity:1034"
      }
    ],
    "coequalScope": true,
    "body": "formula:1091"
  }
}
```

Selbri variables (`bu'a`, `bu'e`, `bu'i`) use the same formula-level
quantifier shape, but `variable` points at a `parameter` object with
`sort = "relation"` and `role = "relationVariable"` rather than at an entity
referent.  A bare relation variable has implicit existential force, while
`ro bu'a zo'u ... bu'a ...` uses `operator = "forall"` and an explicit
`quantity` object from the prenex.

Operators include:

- `atom`
- `not`
- `scoped`
- `and`
- `or`
- `implies`
- `iff`
- `exclusiveOr`
- `whetherOrNot`
- `connectiveQuestion`
- `respectivelyDistribution`
- `quantifierBundle`
- `exists`
- `forall`
- `none`
- `cardinality`
- `pluralExists`
- `pluralForall`

`connector.locus` is required when surface scope matters.  For example, a
connective between sentences, a connective between predicate tails, and a
connective between arguments can have the same truth table but different
sharing behavior for eventualities, arguments, or quantifier scope.  This is an
amendment from the earlier model.

`respectivelyDistribution` records the truth-conditional zip introduced by
`fa'u` when multiple parallel streams co-vary by index.  Its `body` is the
formula being distributed.  Each entry in `streams` has a `slot` parameter with
`role = "respectiveSlot"` and an ordered `items` list.  Streams that correspond
to quantified witnesses can also carry the source `quantity` and a `restriction`
template over the slot; use `distinctPartition = true` when the quantified stream
is partitioned into distinct witnesses.

```json
{
  "type": "formula",
  "operator": "respectivelyDistribution",
  "body": "formula:1092",
  "streams": [
    {
      "slot": "parameter:1093",
      "items": ["entity:1094", "entity:1013"]
    },
    {
      "slot": "parameter:1095",
      "items": ["entity:1096", "entity:1097"],
      "restriction": "formula:1098",
      "quantity": "quantity:1034"
    }
  ],
  "distinctPartition": true
}
```

For termset cases where the second parallel stream is a complete branch formula
such as CLL 14.133, the branch stream's items may be formulas rather than entity
referents; this preserves the modal/tag content associated with each index while
still exposing the zip.

For logical connectives, `connector.truthTable` is the canonical four-bit truth
table in row order `TT`, `TF`, `FT`, `FF`, using `T` and `F` characters.  It
records the truth function after applying operand negation and SE conversion.
Operand negation is position-sensitive: `na` negates the first operand,
afterthought `nai` negates the second operand, forethought head `nai`
(`ganai ... gi`, `gu'enai ... gi`) negates the first operand, and `gik.nai`
(`... ginai ...`) negates the second operand.  Thus `je` is `TFFF`, `ja` is
`TTTF`, `jo` is `TFFT`, `ju` is the left projection `TTFF`, and `se ju` is the
right projection `TFTF`.
When the connective itself is questioned, omit `truthTable` and use
`connector.parameter`.

When the connective itself is questioned with `je'i`/`gi'i`, the formula
operator is `connectiveQuestion`.  Its `connector` has `parameter` instead of
`truthTable`; the referenced parameter has `sort = "connective"` and
`role = "connectiveQuestion"`.  For example, a tensed connective question such
as `pu je'i ba` has `connector.locus = "tense"`, two child formulas for the
past and future branches, and a connector parameter where an answer such as
`je`, `naje`, or `jenai` would be inserted.

Bridi negation `na` is formula-level `not`; it is distinct from scalar
negation on predications.  Prenex or in-bridi `naku` also introduces a
formula-level `not` boundary, with the source preserving whether the boundary
came from the prenex or from the term position inside the bridi.  When a tense
scopes over a non-atomic formula, such as `pu na ...`, use
`operator = "scoped"` with a single child and attach the tense eventuality to
the formula.  Simple tensed predications can keep the time anchor directly on
the predication's eventuality, but nested tense/negation must preserve formula
scope rather than folding the marker into a relation label.

The sharing rules are semantic, not only syntactic:

- `gi'e` connects bridi tails.  An explicit shared x1, including one supplied
  before the first tail, is shared by all connected tails; an omitted x1 is not
  shared and each tail receives its own `zo'e` referent.  Tail terms after the
  connected bridi tail, separated by `vau` when needed, are shared overt terms;
  their argument sources may use `construct = "shared-tail-term"`.  Terms before
  the first connected tail may analogously use `construct = "shared-head-term"`.
- Logical sumti `.e` distributes over the shared predication.  The connected
  argument differs between branches, and overt non-connected arguments are shared
  by id.  Omitted or explicitly elided non-connected places are not shared by
  default; each branch receives its own `zo'e` referent unless the surface gives a
  non-`zo'e` term to share.  If a connected operand is quantified, the
  corresponding distributed branch is wrapped in that operand's quantifier and
  restriction, so `pa mlatu .e pa gerku` preserves both the `pa` cardinalities
  and the `mlatu`/`gerku` restrictions.
- Sentence connectives such as `.ije` join complete formulas; they do not
  inherit the elided-place sharing behavior of sumti connection.
- Logical termset connectives (`pe'e` afterthought and `nu'i` forethought)
  expand into one formula branch per connected termset.  Each branch replays the
  surrounding non-connected terms in surface place order, then fills all omitted
  places independently.  This is why CLL 14.74 has the shared `le briju` in x2
  for the shorter `mi` branch but in x3 for the longer `do ce'e le zarci`
  branch.  Equal-length termsets simply zip their corresponding terms; they do
  not form a Cartesian product.
- Whether-or-not connectives (`.u`/`ju`) keep children in surface order.  The
  truth table records which operand is asserted; the non-asserted operand's
  predications use `mode = "inert"`.  `se ju` therefore makes the first
  surface branch inert and the second asserted.

Non-logical statement connectives such as `.i joi` and `.i ce'o` do not create
formula connectives.  They stay on the `sequence` as:

```json
{
  "type": "sequence",
  "items": ["utterance:1004", "utterance:1005"],
  "nonlogicalConnection": {
    "operator": "mass",
    "connector": {
      "source": "joi",
      "locus": "statement"
    }
  }
}
```

### tanru lowering

A tanru is not lowered by asserting both component predicates, because CLL
does not fix one universal seltau-tertau relation.  Use one uniform schema:
assert the tertau predication, reify the seltau as a `ka` property, and connect
the tertau x1 to that property through a vague tanru relation.

For `ta cinfo kerfa`:

```json
{
  "type": "formula",
  "operator": "and",
  "children": ["formula:1099", "formula:1100"],
  "connector": { "source": "tanru", "locus": "selbri" }
}
```

```json
{
  "type": "predication",
  "relation": "kerfa",
  "eventuality": "eventuality:18",
  "arguments": {
    "x1": { "kind": "filled", "value": "entity:1101" },
    "x2": {
      "kind": "elided",
      "value": "entity:1056",
      "introducedBy": "zo'e"
    },
    "x3": {
      "kind": "elided",
      "value": "entity:1057",
      "introducedBy": "zo'e"
    }
  },
  "mode": "asserted"
}
```

```json
{
  "relation:24": {
    "type": "referent",
    "sort": "relation",
    "parameters": ["parameter:1102"],
    "arity": 1,
    "body": "formula:1103"
  }
}
```

```json
{
  "type": "predication",
  "relation": "tanru",
  "tanruLink": {
    "head": "predication:1104",
    "modifier": "relation:1105",
    "relationLabel": "cinfo-kerfa"
  },
  "arguments": {
    "x1": { "kind": "filled", "value": "entity:1101" },
    "x2": { "kind": "filled", "value": "relation:1105" }
  },
  "mode": "asserted"
}
```

This asserts that the referent is a mane and stands in some tanru relation to
the property of being a lion.  It does not assert `cinfo(entity:1101)`, and
it does not create a separate concrete lion referent.  Intersective readings
resolve the vague relation to instantiation; asymmetric readings resolve it
to possession, material, resemblance, purpose, source, or another contextual
relation.

Tanru inversion with `co` uses the same schema after restoring semantic order.
`B co A` lowers as `A B`: `B` remains the tertau and supplies the predication
and public place structure, while `A` becomes the property modifier.  Sumti
following the inverted selbri fill places of the seltau, not numbered places
of the enclosing bridi.  For example, `mi troci co klama le zarci le zdani`
has an asserted `troci` predication with its x2 and x3 elided, plus a tanru
modifier property whose `klama` body has x2 filled by `le zarci` and x3 filled
by `le zdani`.  The formula connective for this case uses
`connector.locus = "selbri-inversion"` so consumers can see that the surface
used `co` without changing the semantic graph shape.

Logical connectives inside a tanru modifier do not create a vague relation
between the connected modifier words themselves.  They lower inside the `ka`
property body as formula connectives over the same property parameter.  Thus
`barda je xunre gerku` uses the usual tanru link from the dog to a property
whose body is `and(barda(ce'u), xunre(ce'u))`.

Logical connectives can also appear in the tertau position.  In `melbi cmalu
nixli je ckule`, the connected tertau lowers as a formula connective over
`nixli` and `ckule` sharing the visible x1; it must not become an opaque
relation label `nixli je ckule`.

SE conversion affects which place is visible as the x1 of a selbri or tanru
unit.  The tanru relation attaches to that visible x1.  Thus `cadzu se klama`
links the `cadzu` property to the visible x1 of `se klama` (the destination),
while `se ke cadzu klama ke'e` first forms the `cadzu klama` relation and then
converts the whole tanru, so the `cadzu` modifier still applies to the original
`klama` x1.

Scalar negation scopes to the relation selected by NAhE.  In `na'e cadzu
klama`, the tertau `klama` remains asserted and the `cadzu` property inside
the tanru modifier has `scalarNegation`.  In `na'e ke cadzu klama ke'e`, the
whole grouped `cadzu klama` relation has `scalarNegation`; the component
relations are not separately asserted.  The same scoped rule applies across
`bo`, `be`, and omitted terminators, because those determine which relation the
NAhE marker modifies.

Non-logical JOI-series connectives inside a tanru modifier do not enter the
formula layer.  For `blanu joi xunre bolci`, the tertau predication is `bolci`,
and the tanru link points to a composite concept:

```json
{
  "type": "predication",
  "relation": "tanru",
  "tanruLink": {
    "head": "predication:1106",
    "modifier": "entity:1107",
    "relationLabel": "blanu joi xunre-bolci"
  },
  "arguments": {
    "x1": { "kind": "filled", "value": "entity:1101" },
    "x2": { "kind": "filled", "value": "entity:1107" }
  },
  "mode": "asserted"
}
```

Non-brivla selbri that participate in tanru keep their own relation labels and
place structures rather than collapsing to generic placeholders:

- `nu'a su'i` is an open-place operator relation whose x1 is the result and
  whose following places are operands.
- `pa moi` has ordinal places: x1 is the nth member, x2 is the ordered set,
  and x3 is the ordering rule.
- `re mei` has cardinal places: x1 is the mass, x2 is the set of members, and
  x3 is one or more members.
- `nu zdile` reifies the embedded bridi as an event abstraction and links the
  exposed x1 to it with `eventOf`; it is not a lexical relation label
  `"nu zdile"`.
- `me SUMTI` lowers to `referentOf(x1, source)`, where x2 is the fixed source
  referent built from the embedded sumti.  This x2 is part of the public
  semantic relation and is not a user-fillable surface place.  `me'u` changes
  which sumti is embedded: without `me'u`, following connectives can be inside
  the source sumti; with `me'u`, they can connect the resulting outer sumti.

Tanru inside descriptions use the same uniform lowering as asserted selbri.
Thus `loi nu'a su'i nabmi` has a description body equivalent to
`nabmi(x) AND tanruLink(x, ka nu'a su'i ce'u)`, with the `loi` referent
sorted as `mass`.

### assigned pro-bridi

`cei` assigns a pro-bridi handle, but the public JSON should not expose a
definition or binding object.  Later assigned pro-bridi occurrences lower to
the target semantics directly.

When a pro-bridi is used as a bridi, inherited argument places from the
antecedent bridi are carried forward and explicit arguments on the current
bridi override them.  For example, `mi klama cei brode le zarci .i do brode`
emits a second `klama` predication whose x1 is the addressee and whose x2 is
the same store referent from the antecedent bridi.

When a pro-bridi is used inside a description or tanru, it behaves like the
assigned selbri in that local context.  In `le crino broda` after
`ti slasi je mlatu bo cidja lante gacri cei broda`, the described green
referent fills the visible x1 of the expanded assigned selbri; the output does
not contain a bare unknown `broda` predication.

The GOhA series follows the same public rule when the antecedent is resolved:
`go'i`, `go'e`, and similar pro-bridi do not produce binding or expansion-trace
objects.  The repeated bridi is emitted as the antecedent relation with the
antecedent's non-overridden arguments inherited, explicit current arguments
overriding those places, and inherited tense/space anchors kept unless the
current pro-bridi supplies its own anchors.  Vague forms such as `go'o` remain
vague rather than being guessed.

Quotation establishes a quote-local anaphora stream for resolved GOhA and
sumti anaphora.  A pro-bridi inside a quotation cannot see the supporting
narrative bridi outside the quote, but it can see a previous related quotation.
Thus in `la .djan. cusku lu mi klama le zarci li'u .i la .alis. cusku lu mi
go'i li'u`, the second quotation's `go'i` repeats the first quoted `klama`,
not the outer `cusku` bridi.

### abstraction

An abstractor reifies the embedded bridi as an object of the abstractor's
output sort.  The public JSON does **not** wrap that output in a separate
`abstraction` object.  The output object itself carries the embedded formula,
the output sort, any bound parameters, and any real extra abstractor places.

This avoids the older indirection:

```text
entity:1011 --eventOf--> removed abstraction wrapper --body--> formula:1074
```

where the removed wrapper only repeated the output kind and pointed at the body
formula.  That shape also produced two unconnected event-like objects for
`lo nu brode`: the inner predication's eventuality and the described
`entity:1011`.  In the direct shape, `nu` and the aktionsart abstractors
produce the eventuality object that the embedded predication is about.

For event abstractors, the output is a `type:"referent"` object whose `sort`
is an eventuality sort path:

- `eventuality` for broad `nu`, in the CLL sense that includes states,
  processes, activities, and point-events;
- `eventuality/achievement` for `mu'e`;
- `eventuality/process` for `pu'u`;
- `eventuality/activity` for `zu'o`;
- `eventuality/state` for `za'i`;
- `eventuality/experience` for `li'i`;
- `eventuality/locution` for utterance/vocative/fragment locution events.

Every full predication still has an eventuality slot.  A bare bridi uses the
broad `eventuality` sort unless a more specific construction reifies that bridi
through `za'i`, `pu'u`, `zu'o`, or `mu'e`.  The distinction between a bare bridi
and `nu broda` is therefore not the existence or broad sort of the eventuality;
it is that `nu broda` makes that eventuality available as a sumti value.

For example, `lo nu do klama` can denote the same eventuality that fills the
embedded `klama` predication's event slot:

```json
{
  "eventuality:14": {
    "type": "referent",
    "sort": "eventuality",
    "category": "constant",
    "content": "formula:1109",
    "descriptor": {
      "kind": "veridicalDescription",
      "word": "lo"
    },
    "source": { "text": "lo nu do klama", "construct": "description" }
  },
  "predication:1110": {
    "type": "predication",
    "eventuality": "eventuality:14",
    "relation": "klama",
    "arguments": {
      "x1": { "kind": "filled", "value": "entity:2" }
    },
    "mode": "inert"
  }
}
```

When a bridi is reified under multiple connected event abstractors, each
abstractor branch may have its own inert body formula and eventuality object,
because `pu'u broda` and `za'i broda` are different views of the bridi.  Do not
force one eventuality object to carry mutually exclusive event-type classes.

For non-event abstractors, the output is also a `type:"referent"` object of the
appropriate sort with the body formula directly attached.  Public JSON does not
emit `abstractionKind`; the sort path plus `body`/`content`, parameters, and
extra fields carry the semantics.

| abstractor | output sort | required fields |
| --- | --- | --- |
| `ka` | `relation` | `body`, `parameters`, `arity` |
| `ni` | `amount` | `body`, optional `scale` |
| `jei` | `truthValue` | `body`, optional `epistemology` |
| `du'u` | `proposition` | `body`, optional `expressedBy` |
| `si'o` | `concept` | `body`, optional `mind` |
| `li'i` | `eventuality/experience` | `content` or `body`, optional `experiencer` |
| `su'u` | `abstractNature` | `body` |

Property outputs use `parameters`:

```json
{
  "relation:21": {
    "type": "referent",
    "sort": "relation",
    "parameters": ["parameter:22"],
    "arity": 1,
    "body": "formula:1111"
  }
}
```

For `ka`, each distinct `ce'u` introduces a distinct parameter.  The
output's `arity` is the number of distinct `ce'u` parameters: one `ce'u`
is a one-place property, two `ce'u` form a two-place relation, and so on.
When a `ka` abstraction has no explicit `ce'u`, CLL 11.4 treats the first
unfilled surface place as an implicit property focus.  Emit the same kind of
parameter inside the body formula, with `introducedBy = "implicit ce'u"`.
Surface place order matters after conversion: `ka se risna` fills the raw
`risna` x2 slot, because that is the visible x1 of `se risna`, while raw
`risna` x1 remains an elided heart.

```json
{
  "parameter:1112": {
    "type": "parameter",
    "sort": "entity",
    "role": "propertySlot",
    "introducedBy": "implicit ce'u"
  },
  "predication:1048": {
    "type": "predication",
    "relation": "risna",
    "arguments": {
      "x1": {
        "kind": "elided",
        "value": "entity:1011",
        "introducedBy": "zo'e"
      },
      "x2": { "kind": "filled", "value": "parameter:1112" }
    },
    "mode": "restrictive"
  },
  "relation:24": {
    "type": "referent",
    "sort": "relation",
    "parameters": ["parameter:1112"],
    "arity": 1,
    "body": "formula:1074"
  }
}
```

`ke'a` is different: it reuses the relative-clause head rather than introducing
fresh independent parameters.

Additional CLL surface places are fields or arguments on the output object.
They are **not** shifted through an artificial x2 occupied by an abstraction
wrapper.  CLL 11.13 gives these extra places:

- `ni` x2, the measurement scale, appears as `scale` on the quantity output.
- `jei` x2, the epistemology, appears as `epistemology` on the truth-value
  output.
- `li'i` x2, the experiencer, appears as `experiencer` on the experience
  output.
- `si'o` x2, the mind, appears as `mind` on the concept output.
- `du'u` x2, the sentence or text expressing the bridi, appears as
  `expressedBy` on the proposition output.

CLL 11.13 gives no extra x2 for `su'u`; do not fabricate one.

Connected abstractors, as in `pu'u jenai za'i`, produce a connected formula
over the direct output objects.  Logical negation on the connective wraps the
affected branch with a formula whose `operator` is `not`; the connective
formula records `connector.locus = "abstraction"` and preserves the surface
connector text such as `je nai`.

`du'u` also has a CLL-defined x2 for the sentence or text expressing the
predication.  A description headed by `se du'u` therefore describes the text or
sentence sign that expresses the embedded proposition.  The described text's
descriptor points to the proposition output as its operand or uses the
`sentenceExpresses` relation when a predication is needed for a restrictive
description; no separate proposition abstraction wrapper is introduced.

For example, after `mi klama le zarci`, `le si'o mi go'i` describes a concept
whose body is the inert expanded `klama` formula:

```json
{
  "concept:24": {
    "type": "referent",
    "sort": "concept",
    "body": "formula:1109",
    "mind": {
      "kind": "elided",
      "value": "entity:1113",
      "introducedBy": "zo'e"
    },
    "descriptor": {
      "kind": "speakerDescription",
      "word": "le"
    }
  }
}
```

When a NU abstraction is used directly as a selbri, a predication shape may
still be required by the grammar.  In that case the x1 is the direct
abstraction output object, and the body remains attached to that object rather
than to a wrapper:

```json
{
  "eventuality:14": {
    "type": "referent",
    "sort": "eventuality",
    "category": "constant",
    "content": "formula:1109",
    "source": { "text": "nu mi klama le zarci", "construct": "abstraction" }
  }
}
```

When such a NU abstraction is the seltau of a tanru, the seltau property
abstracts over the same output sort.  For `nu sonci kei djica`, the tertau
`djica` remains the asserted predication.  The tanru modifier is a property
whose parameter is an eventuality and whose body is the direct embedded
eventuality output with `content` pointing at the inert `sonci` formula.

The more specific event abstractors use the direct-output pattern with
type-specific classes.  Thus `pu'u` exposes its surface x2 as a `stages` field
or equivalent argument on the process eventuality:

```json
{
  "eventuality/process:14": {
    "type": "referent",
    "sort": "eventuality/process",
    "category": "constant",
    "content": "formula:1079",
    "stages": {
      "kind": "elided",
      "value": "entity:1114",
      "introducedBy": "zo'e"
    }
  },
  "formula:1079": { "type": "formula", "operator": "atom" }
}
```

Self-referential pro-bridi inside an abstraction can make a copied argument
contain the very pro-bridi being expanded, as in `le nu nei`.  The public graph
must stay finite.  In that case the recursive inherited place is emitted as an
ordinary elided `zo'e` argument with a diagnostic on the copied predication;
non-recursive inherited places are still copied.

The zo'e-series distinguishes three public argument shapes.  `zo'e`, whether
explicit or omitted, is `kind = "elided"` and carries an elided referent.
`zi'o` is `kind = "deleted"` and carries no referent.  `zu'i` is neither: it is
a filled argument whose referent descriptor has `kind = "typicalPlaceValue"`
and `word = "zu'i"`, because CLL defines it as the typical value for that
place rather than an unspecified omitted value.

Indirect questions inside `du'u` use `embeddedQuestions`; the question object
itself records the focus parameter and any presupposed answer:

```json
{
  "type": "referent",
  "sort": "proposition",
  "body": "formula:1115",
  "embeddedQuestions": ["question:1116"]
}
```

The earlier model used `RFY`/`abstraction` as a wrapper.  The current public
JSON keeps the reification content but places it on the output object itself,
so `kau`, `ce'u`, scale, mind, experiencer, and expressed-by information are
not lost and the graph does not contain an otherwise-semantic-empty wrapper.

### sign

A sign object represents words, quotations, lerfu, opaque text, and structured
text values.

When a quotation is used as a sumti, the argument filler points directly at the
`sign` object.  It should not be wrapped in an anonymous referent merely to make
it look entity-like, because a quote fills places as a text/sign value.

```json
{
  "type": "sign",
  "kind": "quotation",
  "quotation": {
    "mode": "parsed",
    "utterance": "utterance:1117"
  }
}
```

Opaque quotation:

```json
{
  "type": "sign",
  "kind": "quotation",
  "quotation": {
    "mode": "opaque",
    "delimiter": "gy",
    "text": "lojban.org"
  }
}
```

For non-quotation word or text signs, use top-level `text` as the semantic
payload.  This is distinct from `source.text`, which is only provenance:

```json
{
  "type": "sign",
  "kind": "text",
  "text": "lojban"
}
```

Signs whose surface text has structured semantic content may use `denotes` to
point at that content.  Math-expression signs use the same `text` payload plus
`denotes` for the structured expression:

```json
{
  "type": "sign",
  "kind": "mathExpression",
  "text": "re su'i re",
  "denotes": "math:1030"
}
```

Selbri-based name signs, as in `la gleki` or `la melbi kanla`, likewise
preserve the displayed name text while `denotes` points at the lowered
selbri-derived relation output.  The `cmene` clause uses the sign as x1 and the
named referent as x2; the denoted relation is not thereby asserted of the named
referent.

Standalone logical or non-logical connectives used as answers are signs with
`kind = "connective"`.  Without discourse context, the sign records the
connective choice itself rather than fabricating the omitted question:

```json
{
  "type": "sign",
  "kind": "connective",
  "text": "gi'e nai"
}
```

Letteral signs need more structure than the earlier model provided:

```json
{
  "type": "sign",
  "kind": "letteral",
  "text": "tanru",
  "letterals": [
    {
      "kind": "glyph",
      "sourceWords": ["ty"],
      "text": "ty",
      "value": "t"
    },
    {
      "kind": "glyph",
      "sourceWords": ["a", "bu"],
      "text": ".abu",
      "value": "a",
      "buDepth": 1
    }
  ],
  "source": {
    "span": { "byteStart": 0, "byteEnd": 21 },
    "text": "ty. .abu ny. ry. .ubu",
    "construct": "letteral"
  }
}
```

`letterals[].kind` values include:

- `glyph`
- `digit`
- `shift`
- `characterCode`
- `compound`

Each unit records `sourceWords`, so `BU`-derived lerfu such as `.abu` and
`ky.bu` remain distinct from bare `by`-style lerfu even when a display `value`
is available.  `shift` units preserve case/script/font shifts such as `ga'e`,
`tau`, `zai`, and `ce'a`.  `compound` units preserve `tei`...`foi` grouping
with nested `parts`.  `characterCode` units preserve `se'e` code forms without
claiming a particular character set or radix unless the discourse supplies
that convention.

`text` is a display label for the whole sign.  It may be a simple rendered
sequence such as `tanru` when the letteral values are unambiguous; otherwise it
falls back to the surface expression.  Do not put semantic letteral structure
in top-level `source`; `source` remains provenance only.

In sumti position, a lerfu string can instead be a pro-sumti.  When reference
analysis resolves it, emit the resolved referent directly.  For example, after
`le gerku`, `gy.` in sumti position points at that dog referent rather than at
a separate letteral-sign object.  If a letteral pro-sumti cannot be resolved,
emit a diagnostic referent rather than fabricating an antecedent.

Multi-lerfu pro-sumti can resolve by a multi-initial key.  CLL 17.9's
`symydy. tavla .abupyky.` after the names Steven Mark Jones and Alexander
Pavlovich Kuznetsov therefore points directly at those two name referents in
the public graph.

### displayedContent

Displayed content is semantically visible but not normally a truth conjunct of
the host formula: attitudinals, evidentials, discursives, focus markers,
emphasis, and metalinguistic operators live here.

```json
{
  "type": "displayedContent",
  "family": "emotion",
  "relation": "happy",
  "experiencer": "entity:1",
  "target": "eventuality:18",
  "anchor": "utterance:1004",
  "intensity": null,
  "polarity": "positive",
  "phase": null,
  "modifiers": [],
  "assertionEffect": "none"
}
```

Families include:

- `emotion`
- `attitudeModifier`
- `propositionalAttitude`
- `evidential`
- `discursive`
- `metalinguistic`
- `emphasis`
- `questionPrompt`

`assertionEffect` records whether the displayed content remains projective or
acts performatively:

- `none`
- `hostAsserted`
- `hostSubordinated`
- `metalinguisticallyVoided`
- `performative`

Leading propositional attitudinals such as `.a'o` target the host formula and
anchor to the utterance.  Because CLL 13.3 treats propositional attitudes as
subordinating the host proposition rather than asserting it outright, they use
`assertionEffect = "hostSubordinated"`.  When a displayed-content object is
anchored to an utterance and is not itself the utterance content, include it in
that utterance's `asides` so the graph remains reachable from `root`.

```json
{
  "display:1118": {
    "type": "displayedContent",
    "relation": "hope",
    "family": "propositionalAttitude",
    "polarity": "positive",
    "assertionEffect": "hostSubordinated",
    "experiencer": "entity:1",
    "target": "formula:1119",
    "anchor": "utterance:1004",
    "source": {
      "span": { "byteStart": 1, "byteEnd": 4 },
      "text": "a'o",
      "construct": "indicator"
    }
  }
}
```

Metalinguistic `na'i` uses `family:"metalinguistic"` and
`assertionEffect:"metalinguisticallyVoided"`.  It does not emit formula
negation; the host predication is marked inert because the utterance is being
challenged as mis-posed rather than asserted as true or false.  When the same
formula can be targeted at different levels, `targetFocus` records the intended
surface focus: leading `na'i go'i` has `targetFocus:"bridi"`, while post-selbri
`go'i na'i` has `targetFocus:"selbri"`.

```json
{
  "type": "displayedContent",
  "relation": "metalinguisticNegation",
  "family": "metalinguistic",
  "target": "formula:1120",
  "targetFocus": "selbri",
  "anchor": "utterance:1004",
  "assertionEffect": "metalinguisticallyVoided"
}
```

Compound indicators are represented as one displayed-content object when the
extra indicators modify a preceding base indicator.  The base indicator remains
the `relation`; modifier words go in `modifiers` so they are not confused with
independent attitudes.  Each modifier has a `relation`, and may also carry
`polarity` or `intensity` when the modifier itself is negated or graded.

```json
{
  "display:1118": {
    "type": "displayedContent",
    "relation": "desire",
    "family": "propositionalAttitude",
    "polarity": "positive",
    "modifiers": [
      {
        "relation": "selfOrientation",
        "polarity": "negative"
      }
    ],
    "assertionEffect": "hostSubordinated",
    "experiencer": "entity:1",
    "target": "formula:1074",
    "anchor": "utterance:1004",
    "source": {
      "text": "ause'inai",
      "construct": "indicator"
    }
  }
}
```

`pei` turns the displayed content into an attitude question.  If it modifies a
base attitude such as `.iepei`, the public object is a `questionPrompt` display
whose relation names the requested attitude domain, for example
`agreementQuestion`.  A standalone indicator-only text is still an utterance:
its content is the displayed-content object, and the target is a `sign` holding
the indicator expression.

`dai` shifts the experiencer away from the speaker to an elided empathetic
experiencer.  It is not a separate displayed attitude.  For example,
`.oiro'odai` emits one complaint display with a `physical` modifier and an
elided `experiencer` referent introduced as the empathetic experiencer.

When a modifier word appears without a preceding base attitudinal in the same
indicator group, emit it as `family = "attitudeModifier"` rather than using the
raw cmavo as a relation.  Thus `ko ga'inai ...` displays `relation = "rank"`
with negative polarity targeting the addressee, and `le cukta be'u ...`
displays `relation = "need"` targeting the book referent.

Indicators can target discourse acts as well as formula-bearing content.  A
leading indicator on a vocative-only text, such as `ru'a doi .livinston.`, uses
the vocative utterance itself as both `target` and `anchor`.  An indicator on a
bare `.i` separator with no following statement, such as `.i .e'a`, targets the
previous discourse item's content and anchors to that previous utterance.

### mathExpression

Math expressions preserve operator and operand structure.

```json
{
  "type": "mathExpression",
  "operator": "add",
  "operands": ["math:1121", "math:1122"]
}
```

Literal:

```json
{
  "type": "mathExpression",
  "literal": {
    "kind": "integer",
    "value": 4
  }
}
```

Simple PA integers are numeric literals; simple PA decimals are preserved as a
decimal textual literal until the model grows an arbitrary-precision decimal
numeric value:

```json
{
  "math:1123": {
    "type": "mathExpression",
    "literal": { "kind": "integer", "value": 87 }
  },
  "math:1122": {
    "type": "mathExpression",
    "literal": { "kind": "decimal", "value": "0.5" }
  }
}
```

PA word runs containing `pi'e` are mixed-radix or positional literals, not
ordinary arithmetic expressions.  Preserve their ordered components in a
structured literal.  Components keep their surface token text and include an
integer value when the component is exactly understood as a simple PA integer.
`ju'u` base/radix metadata is a future extension on the same literal shape.

```json
{
  "math:1124": {
    "type": "mathExpression",
    "literal": {
      "kind": "mixedRadix",
      "value": {
        "components": [
          { "text": "pa pi re" },
          { "text": "ze", "integer": 7 }
        ]
      }
    }
  }
}
```

The v0 output for `li vo su'i re du li xa` now preserves `4 + 2 = 6`; the JSON
model should do the same.  Arithmetic must not collapse into anonymous
existentials.

When the mekso operator itself is questioned, the math expression carries an
`operatorParameter` instead of an `operator` string:

```json
{
  "parameter:1125": {
    "type": "parameter",
    "sort": "mathOperator",
    "role": "mathOperatorQuestion",
    "introducedBy": "mo"
  },
  "math:1124": {
    "type": "mathExpression",
    "operatorParameter": "parameter:1125",
    "operands": ["math:1123", "math:1122"]
  }
}
```

When `na'u <selbri>` supplies a concrete selbri operator, the math expression
still records the operator label but also carries `operatorDenotes`, pointing
at the lowered relation or property output for the selbri.  `na'u mo` remains
the question case and uses `operatorParameter`.

```json
{
  "math:1124": {
    "type": "mathExpression",
    "operator": "tanjo",
    "operatorDenotes": "relation:31",
    "operands": ["math:1123", "math:1122"]
  },
  "relation:31": {
    "type": "referent",
    "sort": "relation",
    "body": "formula:1126"
  }
}
```

Mekso intervals use the same endpoint-inclusion vocabulary as non-logical
sumti interval compositions:

```json
{
  "type": "mathExpression",
  "operator": "orderedInterval",
  "endpointInclusion": { "left": "inclusive", "right": "exclusive" },
  "operands": ["math:1127", "math:1128"]
}
```

Scalar-negated operators and operands use `scalarNegation` on the affected
math expression:

```json
{
  "math:1124": {
    "type": "mathExpression",
    "operator": "add",
    "scalarNegation": { "kind": "otherThan", "introducedBy": "na'e" },
    "operands": ["math:1123", "math:1122"]
  },
  "math:1121": {
    "type": "mathExpression",
    "literal": { "kind": "integer", "value": 5 },
    "scalarNegation": { "kind": "otherThan", "introducedBy": "na'e" }
  }
}
```

`xi` attaches as `subscript` on referents, parameters, signs, or math
expressions.  `subscript.value` points at a math expression, and
`introducedBy` preserves the source marker.  Subscripts are identity-relevant
for variable-like handles, so a fully identity-aware implementation must keep a
subscripted handle distinct from its bare form before anaphora resolution.

```json
{
  "math:1123": {
    "type": "mathExpression",
    "literal": { "kind": "variable", "value": "x" },
    "subscript": {
      "value": "math:1122",
      "introducedBy": "xi"
    }
  },
  "math:1122": {
    "type": "mathExpression",
    "literal": { "kind": "integer", "value": 2 }
  }
}
```

When a mekso operand is a sumti introduced by `mo'e`, use a math-expression
leaf whose literal marks the conversion and whose `denotes` field points at the
sumti referent.  This keeps the arithmetic expression in math-expression space
without losing the semantic object that supplies the operand value.

```json
{
  "math:1122": {
    "type": "mathExpression",
    "literal": { "kind": "sumtiOperand", "value": "mo'e" },
    "denotes": "entity:1129"
  },
  "math:1124": {
    "type": "mathExpression",
    "operator": "subtract",
    "operands": ["math:1123", "math:1122"]
  }
}
```

When a mekso operand is a selbri introduced by `ni'e`, use a math-expression
leaf whose literal marks the conversion and whose `denotes` field points at the
direct semantic output supplied by that selbri when one is explicit.  The
wrapped selbri is still lowered structurally by the shared selbri-wrapper rule:
`ni'e ni clani` denotes the amount output, while a non-abstraction selbri must
preserve the predication body that constrains its result slot rather than
collapsing to opaque text.

```json
{
  "amount:41": {
    "type": "referent",
    "sort": "amount",
    "body": "formula:1130"
  },
  "math:1123": {
    "type": "mathExpression",
    "literal": { "kind": "selbriOperand", "value": "ni'e" },
    "denotes": "quantity:1131"
  }
}
```

SE conversion on mekso operators is reflected by the operand order in the
resulting math expression.  For example, `ci se vu'u vo` is subtraction with
the first two operands swapped, representing `4 - 3`, not `3 - 4`.

Logical connectives between mekso operators are not serialized as fused operator
strings.  When such a connected operator occurs in a `li ... du ...` identity
claim, the identity claim branches at formula level.  For CLL 14.151/14.152,
`li re su'i je pi'i re du li vo` becomes an `and` formula with
`connector.locus = "mekso-operator"` over two identity atoms: one for
`re su'i re = vo` and one for `re pi'i re = vo`.

### quantity

A quantity object represents quantifier values and count expressions.  It is
separate from `mathExpression` because Lojban quantity words include exact,
approximate, comparative, indefinite, adequate, and scale-sensitive values.

```json
{
  "type": "quantity",
  "form": "exact",
  "value": { "integer": 2 },
  "scale": "count"
}
```

```json
{
  "type": "quantity",
  "form": "approximate",
  "value": { "integer": 5 },
  "scale": "count",
  "approximation": "near"
}
```

Fields:

- `form`: `exact`, `all`, `atLeast`, `atMost`, `moreThan`, `lessThan`,
  `approximate`, `indefinite`, `enough`, `tooMany`, `tooFew`
- `scale`: `count`, `fraction`, `ordinal`, `amount`, `extent`, `frequency`
- `value`: a literal or a `mathExpression`
- `comparisonSet`: optional reference for relative quantities

This is a new object type.  The earlier model had `MEX` and quantity-sorted
referents, but v0's quantity prelude showed that quantifier semantics needs a
dedicated object shape.

### question

A question object represents an answer domain plus the body that constrains a
valid answer.  It is used for direct questions (`xu`, `ma`, `mo`, `fi'a`,
connective questions, tense questions, `pei`) and embedded `kau` questions.

```json
{
  "type": "question",
  "kind": "truth",
  "mode": "direct",
  "asker": "entity:1",
  "respondent": "entity:2",
  "domain": "truthValue",
  "body": "formula:1132"
}
```

Argument question:

```json
{
  "type": "question",
  "kind": "argument",
  "mode": "direct",
  "asker": "entity:1",
  "respondent": "entity:2",
  "domain": "entity",
  "body": "formula:1132",
  "slots": [
    { "parameter": "parameter:1133", "role": "answer" }
  ]
}
```

The body formula contains `parameter:1133` where the answer should be inserted.

Indirect question with a concrete marked focus:

```json
{
  "type": "question",
  "kind": "argument",
  "mode": "indirect",
  "domain": "entity",
  "body": "formula:1134",
  "focus": "entity:1003",
  "presupposedAnswer": "entity:1003"
}
```

For `ma kau`, `focus` would point at a parameter and `presupposedAnswer` would
be absent.

For direct `cu'e`, use `kind = "tense"` and `domain = "tenseModal"`; the body
contains an eventuality with `tenseModal` pointing at the answer parameter.
For direct `je'i`, use `kind = "connective"` and `domain = "connective"`; the
body contains a `connectiveQuestion` formula whose connector points at the
answer parameter.

`kind` values include:

- `truth`
- `argument`
- `relation`
- `place`
- `connective`
- `tense`
- `attitude`
- `quantity`

This is the largest object-model amendment from the v0 review.  The v0 prelude
and GitLab planning issues converged on a generic question model with typed
answers.  The JSON graph should preserve the same semantic fact without
encoding it as a Lean `Question alpha`.

### relationMetadata

Relation metadata describes lexical or constructed relations without asserting
their expansion as discourse truth.

```json
{
  "type": "relationMetadata",
  "relation": "seljvajvo",
  "sourceWords": ["se", "jvajvo"],
  "placeStructure": [
    { "place": "x1", "description": "lujvo" },
    { "place": "x2", "description": "source tanru" }
  ],
  "expansion": {
    "kind": "lujvo",
    "sourceWords": ["se", "jvajvo"],
    "placeIdentifications": [],
    "rafsiBindings": []
  }
}
```

Do not use relation metadata to silently desugar a lujvo into asserted
component relations.  It is lexical explanation, not the main semantic content
unless the source text explicitly asserts it.

When a lujvo contains a rafsi whose source word is itself context-sensitive,
such as a pro-sumti rafsi, preserve that resolution inside
`expansion.rafsiBindings`:

```json
{
  "type": "relationMetadata",
  "relation": "fo'arselsanga",
  "sourceWords": ["fo'a", "se", "sanga"],
  "expansion": {
    "kind": "lujvo",
    "sourceWords": ["fo'ar", "sel", "sanga"],
    "rafsiBindings": [
      {
        "rafsi": "fo'ar",
        "sourceWord": "fo'a",
        "referent": "entity:1011"
      }
    ]
  }
}
```

This records that the lexical relation contains a pro-sumti component resolved
in the current discourse, without asserting the lujvo decomposition as an
additional predication.

## Worked JSON Sketches

These sketches omit source spans and some repeated object definitions to keep
the object shape visible.  Predication arguments still use the public
structured filler shape.

### `lo botpi cu xunre`

```json
{
  "version": "lojban-semantics-json-1",
  "root": "utterance:5",
  "objects": {
    "utterance:5": {
      "type": "utterance",
      "force": "assert",
      "speaker": "entity:1",
      "audience": "entity:2",
      "eventuality": "eventuality/locution:6",
      "content": "formula:1135"
    },
    "entity:1": {
      "type": "referent",
      "category": "indexical",
      "sort": "entity",
      "indexical": "speaker"
    },
    "entity:2": {
      "type": "referent",
      "category": "indexical",
      "sort": "entity",
      "indexical": "audience"
    },
    "entity:1136": {
      "type": "referent",
      "category": "constant",
      "sort": "entity",
      "descriptor": {
        "kind": "veridicalDescription",
        "word": "lo",
        "body": "formula:1023"
      }
    },
    "formula:1023": {
      "type": "formula",
      "operator": "atom",
      "predication": "predication:1137"
    },
    "predication:1137": {
      "type": "predication",
      "relation": "botpi",
      "arguments": {
        "x1": { "kind": "filled", "value": "entity:1136" },
        "x2": {
          "kind": "elided",
          "value": "entity:1138",
          "introducedBy": "zo'e"
        },
        "x3": {
          "kind": "elided",
          "value": "entity:1139",
          "introducedBy": "zo'e"
        },
        "x4": {
          "kind": "elided",
          "value": "entity:1140",
          "introducedBy": "zo'e"
        }
      },
      "mode": "restrictive"
    },
    "formula:1135": {
      "type": "formula",
      "operator": "atom",
      "predication": "predication:1141"
    },
    "predication:1141": {
      "type": "predication",
      "relation": "xunre",
      "eventuality": "eventuality/state:18",
      "arguments": { "x1": { "kind": "filled", "value": "entity:1136" } },
      "mode": "asserted"
    },
    "eventuality/locution:6": {
      "type": "referent",
      "sort": "eventuality/locution",
      "category": "constant"
    },
    "eventuality/state:18": {
      "type": "referent",
      "sort": "eventuality/state",
      "category": "constant",
      "actuality": { "kind": "actual" }
    }
  }
}
```

### `mi klama le zarci .i do cadzu le bisli`

```json
{
  "version": "lojban-semantics-json-1",
  "root": "sequence:1142",
  "objects": {
    "sequence:1142": {
      "type": "sequence",
      "relation": "same-topic-continuation",
      "items": ["utterance:1004", "utterance:1005"]
    },
    "utterance:1004": {
      "type": "utterance",
      "force": "assert",
      "content": "formula:1143"
    },
    "utterance:1005": {
      "type": "utterance",
      "force": "assert",
      "content": "formula:1144"
    }
  }
}
```

The two utterances can both be true, false, questioned, or modified
independently.  The sequence is not the same as `formula:1145`.

### Direct and Indirect Questions

For `xu do nelci le cnino jvinu ...`, the main utterance asks a truth
question:

```json
{
  "type": "utterance",
  "force": "ask",
  "content": "question:1146"
}
```

```json
{
  "type": "question",
  "kind": "truth",
  "mode": "direct",
  "asker": "entity:1",
  "respondent": "entity:2",
  "domain": "truthValue",
  "body": "formula:1147"
}
```

For `ma tavla do mi`, the `ma` parameter fills the x1 place inside the formula:

```json
{
  "type": "question",
  "kind": "argument",
  "mode": "direct",
  "asker": "entity:1",
  "respondent": "entity:2",
  "domain": "entity",
  "body": "formula:1148",
  "slots": [
    { "parameter": "parameter:1133", "role": "answer" }
  ]
}
```

```json
{
  "type": "formula",
  "operator": "atom",
  "predication": "predication:1149"
}
```

```json
{
  "type": "predication",
  "relation": "tavla",
  "eventuality": "eventuality:18",
  "arguments": {
    "x1": { "kind": "filled", "value": "parameter:1133" },
    "x2": { "kind": "filled", "value": "entity:2" },
    "x3": { "kind": "filled", "value": "entity:1" },
    "x4": {
      "kind": "elided",
      "value": "entity:1150",
      "introducedBy": "zo'e"
    }
  },
  "mode": "asserted"
}
```

For `la .djan. kau` inside `du'u`, the question is embedded and has a
presupposed focus value:

```json
{
  "type": "question",
  "kind": "argument",
  "mode": "indirect",
  "domain": "entity",
  "body": "formula:1151",
  "focus": "entity:1003",
  "presupposedAnswer": "entity:1003"
}
```

## Amendments From the v0 Review

These are the semantic object-model changes relative to
`lojban-discourse-object-model.md`, excluding the switch to JSON notation.

1. Added `question` as a first-class object.
   v0's current Lean prelude has a generic typed question API, and generated
   output for `xu ...` uses that shape.  The JSON model should keep direct
   truth questions, argument questions, `kau`, `fi'a`, connective questions,
   tense questions, and `pei` as explicit answer-domain objects.

2. Added `quantity` as a first-class object.
   v0 has a substantial quantity and math prelude.  Quantifier values are not
   always plain integers or math expressions, so JSON needs a quantity object
   for exact, approximate, comparative, indefinite, adequate, fractional,
   ordinal, extent, and frequency values.

3. Expanded referents with `composition`.
   Masses, sets, sequences, respectively-mapped groups, unions, intersections,
   and Cartesian products are now modeled as composite referents.  This follows
   the useful parts of v0's `massOf`, `setOf`, `sequenceOf`,
   `respectivelyMap`, `setUnion`, `setIntersection`, and `crossProduct`
   helpers without adopting those names as required surface JSON.

4. Expanded `sign.source`.
   Letterals and quotations need structured source information for `BU`, nested
   or grouped lerfu, script/font/case shifts, quoted text, and compound signs.
   The earlier `GLYPH/REL/QUOTATION` distinction was too coarse.

5. Added `connector.locus` on formula connectives.
   v0 and the CLL examples both show that the surface position of a connective
   affects sharing and scope.  JSON should not rely on a truth table alone.

6. Replaced string argument values with structured `filled` / `elided` /
   `deleted` fillers.
   The object model now exposes every dictionary-known place.  Omitted places
   are explicit `zo'e` referents; deleted `zi'o` places are explicit deletions
   with no referent and no import.  This prevents the two from being conflated
   in consumers.

7. Added demonstrative indexical referents.
   `ti`, `ta`, and `tu` are now proximal, medial, and distal demonstratives
   instead of opaque pro-sumti constants.  This preserves CLL/v0's deictic
   contrast for examples such as `ta bloti` and `tu pelnimre tricu`.

8. Added abstraction output sorts and `ka` arity.
   `du'u` yields a `proposition`, `jei` yields a `truthValue`, and a `ka`
   abstraction records the number of distinct `ce'u` parameters so a two-place
   relation is not mistaken for a reflexive one-place property.

9. Replaced the earlier tanru split with a uniform tanru schema.
   The tertau predication is asserted, the seltau is reified as a `ka`
   property, and an unresolved tanru link predication links the tertau x1 to
   that property.  This avoids asserting either `S(x)` or a fabricated
   concrete seltau referent.

10. Added tanru inversion lowering for `co`.
   `B co A` is serialized with the same tertau-plus-property schema as `A B`.
   Post-`co` sumti fill the seltau property's places, and the top-level
   connective records `connector.locus = "selbri-inversion"` to preserve the
   surface scope.  This is a graph-construction amendment, not a new object
   type.

11. Added composite concept modifiers for non-logical tanru connectives.
   JOI-series connectives inside a tanru modifier, such as `blanu joi xunre
   bolci`, do not produce formula-level truth connectives.  They produce a
   `referent` with `category = composite`, `sort = concept`, and a
   `composition` over property abstractions, which is then used as the tanru
   relation's modifier argument.

12. Corrected connective sharing.
   `gi'e` shares an explicit x1 only; omitted x1 places are separate `zo'e`
   referents per tail.  Logical sumti `.e` distributes over one shared
   predication, shares overt non-connected arguments, and gives omitted
   non-connected places branch-local `zo'e` referents.

13. Refined eventuality anchors.
   Temporal and spatial tags should point to structured anchors for moments,
   regions, paths, current speech time, and current location, not just flat
   tense/aspect labels.

14. Added explicit number-sumti referents.
   CLL 5.9 uses `li vo` and `li re` as ordinary sumti.  The previous fallback
   made them anonymous unlowered sumti.  They are now `referent` objects with
   `sort = number` and descriptor `quantity`, preserving the numeric value.

15. Added constructed non-brivla selbri labels and arities.
   `nu'a su'i`, `pa moi`, `re mei`, and `nu zdile` are not brivla, but CLL
   gives them specific relation behavior.  JSON now preserves the actual
   operator/MOI/NU marker in the relation label and supplies known places for
   MOI and event abstraction while treating `nu'a` as open-arity.

16. Lowered tanru inside descriptions and sorted mass/set gadri.
   CLL 5.9's `loi nu'a su'i nabmi` forced description bodies to use the same
   uniform tanru schema as asserted selbri, and forced `loi` descriptions to
   be mass referents rather than generic entities.

17. Added `me` as a semantic `referentOf` relation.
   CLL 5.10 uses `me SUMTI` to make a selbri whose x1 is among, equal to, or
   otherwise referentially derived from the embedded sumti.  The previous
   generic relation label could not expose the embedded source once the output
   stopped using Lean helper definitions.  JSON now emits a binary
   `referentOf` predication whose x2 is the fixed embedded source referent.

18. Added descriptor operands for sumti qualifiers.
   CLL 5.10's `me la'e le se cusku be do me'u cukta`, backed by CLL 6.10,
   forced `la'e` to survive as semantics rather than being flattened to its
   inner description.  Referent descriptors now have an `operand` field for
   qualified sumti such as `la'e`, `lu'e`, `lu'a`, `lu'i`, `lu'o`, and `vu'i`.

19. Clarified SE conversion inside tanru lowering.
   CLL 5.11 distinguishes converting an entire grouped tanru from converting
   only one tanru unit.  The tanru relation now attaches to the visible x1 of
   the tertau or converted tanru unit, while whole-tanru conversion preserves
   the unconverted modifier relation before applying the outer place
   conversion.

20. Added modal arguments on predications.
   CLL 5.7 distinguishes `blanu be ga'a mi be'o zdani`, where the observer tag
   scopes to the linked modifier, from `blanu zdani ga'a mi`, where it scopes to
   the enclosing predication.  `modalArguments` records these tagged arguments
   separately from numbered `xN` places.

21. Added occurrence-scoped relative clauses on argument fillers.
   CLL 5.7 uses `xamgu be do noi barda` versus
   `xamgu be do be'o noi barda` to show that a relative clause can attach to a
   linked sumti or to the outer description.  `relativeClauses` on
   `ArgumentValue` records occurrence attachment without mutating the underlying
   referent; description-internal attachment is recorded on the descriptor.

22. Added scalar negation on predications.
   CLL 5.12 distinguishes scalar negation from logical negation and shows that
   NAhE scope can target a single tanru unit or a whole grouped tanru.  The
   previous model either dropped the marker or had to conflate it with a
   relation label.  `scalarNegation` records the NAhE kind and source marker on
   the modified predication.

23. Added scoped formula wrappers for tense over non-atomic formulas.
   CLL 5.13 distinguishes bridi negation `na` from scalar negation and allows
   tense/negation nesting such as `na pu na ca`.  The previous predication-only
   time anchor could not represent a tense whose operand is itself a negated
   proposition.  `operator = "not"` records bridi negation, and
   `operator = "scoped"` lets a formula carry an eventuality/time anchor.

24. Added vocative force and `vocativeKind`.
   Vocatives are utterance-level acts, not ordinary asserted predicates.  The
   v0 prelude already made this distinction, and JSON should preserve it.

25. Made diagnostics explicit for unresolved interpretation.
   v0 retrospective notes recorded several places where placeholder output hid
   missing semantics.  The JSON graph should surface those cases directly with
   diagnostics rather than silently inventing an entity or dropping a marker.

26. Lowered quoted sumti to `sign` objects.
   CLL 6.1 lists quotations as one of the five simple sumti kinds.  The previous
   implementation fallback produced an `unloweredSumti` referent, losing the
   fact that the argument is text/sign-valued.  Parsed `lu ... li'u` quotations
   now emit `sign` objects with `kind = "quotation"` and a nested utterance
   reference when available; opaque quote forms preserve their source text in
   the quotation payload.

27. Resolved `ko` as command force plus addressee argument.
   CLL 6.1 uses `ko` as a pro-sumti in an imperative sentence.  Treating it as
   an opaque `proSumti` constant loses the command semantics.  JSON now resolves
   `ko` directly to `entity:2` in argument position and marks the
   enclosing utterance with `force = "command"`.

28. Allowed mention utterances to carry non-formula content.
   CLL 6.2 uses standalone descriptions such as `le zarci` and `lo zarci` as
   examples.  They are not truth-bearing assertions, but the output should still
   expose the described referent rather than a diagnostic-only fragment.
   `force = "mention"` utterances may therefore point `content` at a referent,
   sign, quantity, or other argument-fillable object.

29. Distinguished LA-series mass and set names.
   CLL 6.3 uses `lai cribe` for a mass of things sharing a name.  The previous
   descriptor kind treated `lai` selbri descriptions as generic entity
   descriptions and cmevla `lai` as ordinary names.  `lai`/`la'i` now produce
   mass/set sorts with `massNameDescription` or `setNameDescription` for
   selbri-based names, and `massName` or `setName` for cmevla names.

30. Added typical and stereotypical descriptor kinds.
   CLL 6.5 distinguishes `lo'e` typical descriptions from `le'e` speaker
   stereotypes.  The previous generic `description` kind hid this contrast.
   JSON now emits `typicalDescription` for `lo'e` and
   `speakerStereotypeDescription` for `le'e`.

31. Preserved branch negation on logical connectives.
   CLL 6.5 uses `na.e` to mean "not the first connected branch and the second".
   A plain `operator = "and"` over both branches loses that semantics.  JSON now
   represents explicit branch negation with a formula-level `not` wrapper around
   the affected child and keeps the full connective marker in `connector.source`.
   For forethought forms, head-side `nai` is first-branch negation (`ganai X gi
   Y`), while `gik.nai` remains second-branch negation (`ge X ginai Y`).

32. Corrected outer quantifier representation.
   CLL 6.6 uses quantified pro-sumti and quantified quotations such as
   `re do cadzu le bisli` and `mi cusku re lu do cadzu le bisli li'u`.  These
   quantifiers cannot be stored by mutating the global addressee referent or the
   quoted sign object, and they also should not be hidden on the argument
   occurrence.  Outer quantifiers are formula-level restricted-variable scopes;
   inner descriptor quantifiers such as the `ci` in `le ci gerku` remain
   descriptor `quantity`.

33. Reused descriptor operands for sumti-based descriptions.
   CLL 6.9 uses `le re do` and `le re le ci cribe`, where a description is
   based on an embedded sumti rather than a selbri.  The previous model could
   preserve the quantity but had nowhere to point at the embedded sumti.
   `descriptor.operand` now records that embedded sumti object; descriptor
   `quantity` records the required inner quantifier, and any outer quantifier
   is represented by a formula-level restricted-variable scope.

34. Allowed nested discourse sequences.
   CLL 6.10 includes multi-statement `.ije` text whose natural syntax is
   left-associated.  Requiring every `sequence.items` entry to be an utterance
   made that graph invalid even though a sequence is itself a discourse unit.
   `sequence.items` may now contain utterance or sequence IDs.

35. Added scalar-negated sumti qualifiers.
   CLL 6.10 uses `na'ebo`, `to'ebo`, and `no'ebo` as sumti qualifiers, distinct
   from scalar negation of a selbri.  The previous builder dropped that layer
   by unwrapping the inner sumti.  Qualified referents now use descriptor kinds
   such as `otherThan`, `oppositeOf`, and `neutralOf`, with
   `descriptor.operand` pointing at the qualified sumti.

36. Represented sentence-internal vocatives as utterance asides.
   CLL 6.11 says that `doi .djan. ko klama mi` and
   `ko klama mi doi .djan.` have the same meaning.  Dropping the free modifier
   loses the addressed-person signal, while turning it into asserted content
   gives it the wrong force.  The existing `asides` field now carries nested
   `force = "vocative"` utterances; bare-selbri vocatives use an implicit
   `speakerDescription` target.

37. Added text payloads for non-quotation signs.
   CLL 6.12 includes bare cmevla examples such as `.lojban.` that are name
   words, not referential `la` sumti.  An empty discourse sequence lost the
   example content, while treating the word as a named referent would add a
   descriptor not present in the Lojban.  Sign objects with `kind = "text"` now
   have a top-level `text` payload for mentioned name-word text.

38. Added universal quantity form `all`.
   CLL 6.13's `ro da poi prenu cu prami pa de poi finpe` forced `ro` to be
   represented as universal quantity rather than as an exact-but-textual count.
   Quantity objects now allow `form = "all"` while keeping the surface value
   text `ro`.

39. Clarified formula-scope lowering for quantificational pro-sumti.
   The same CLL 6.13 example needs quantifier order, not just quantities
   attached to referents or argument occurrences.  `ro da` becomes a `forall`
   formula whose body contains the narrower `pa de` `cardinality` formula; each
   `poi` clause supplies the corresponding formula `restriction`.

40. Added metalinguistic pro-sumti referent targets.
   CLL 6.13's `la'e di'u jetnu` uses `di'u` to refer to the previous utterance.
   An opaque `proSumti` descriptor lost the resolved discourse target.  A
   referent with `descriptor.kind = "utteranceReference"` now carries a
   top-level `target` pointing at the resolved utterance or sequence.

41. Added `vocativeKind = "selfIdentification"`.
   CLL 6.14's `lu mi'e .djan. li'u` contains a parsed quoted text whose only
   semantic content is a `mi'e` vocative attached to the quote marker.  Treating
   it as an opaque vocative kind loses the conventional self-identification
   force.  Parsed quotations therefore preserve marker free modifiers as quoted
   utterances, and `mi'e` uses `selfIdentification`.

42. Distinguished `li` numeric values from `me'o` expression signs.
   CLL 6.15 says that `li re su'i re` denotes the number value while
   `me'o re su'i re` denotes the expression text/sign.  Treating both as
   `sort = "number"` referents erased that contrast.  Complex `li` values now
   use `quantity.value.mathExpression`, while `me'o` emits a `sign` with
   `kind = "mathExpression"`, semantic `text`, and `denotes` pointing at the
   structured `mathExpression`.

43. Added vocative-assignment targets for named referents.
   CLL 7.2 says `mi'e` assigns `mi`, while other vocatives such as `doi`
   assign `do`.  A plain vocative utterance with only an `audience` field did
   not record which indexical the named referent resolved to.  Named referents
   introduced by vocatives now use `target = "entity:1"` for `mi'e`
   and `target = "entity:2"` for address vocatives.

44. Added spatial anchors for selbri distance tags.
   CLL 7.3's `le vi bloti` distinguishes a nearby boat from `le ti bloti`.
   Dropping `vi` after syntax would erase that distinction.  Restrictive and
   asserted predications with `vi`, `va`, or `vu` now get an eventuality whose
   `space` relation is anchored at `entity:4`.

45. Added referent-level relative clauses for standalone sumti mentions.
   CLL 7.3's `ti noi bloti` is a mention of the demonstrative referent with an
   incidental "is a boat" clause.  Argument-level `relativeClauses` cannot
   represent this because there is no enclosing predication argument.  Referent
   objects may therefore carry `relativeClauses` when the relative clause is
   part of the referenced sumti itself.

46. Completed di'u-series utterance references.
   CLL 7.4 distinguishes the previous utterance `di'u`, the current utterance
   `dei`, the following utterance `di'e`, and the unspecified utterance `do'i`.
   The previous model only resolved neighboring utterances after construction,
   so `dei` could not point at the current utterance and `do'i` fell through to
   an opaque entity-valued `proSumti`.  Utterance IDs are now reserved before
   statement content is lowered, letting `dei` target the current utterance;
   `do'i` uses the same `utteranceReference` descriptor with no target.

47. Resolved `goi` assignments directly and added context-local assigned names.
   CLL 7.5 says `ko'a goi la .alis.` and `la .alis. goi ko'a` define the
   assignable pro-sumti as the associated referent, so public JSON uses the
   associated referent ID directly rather than a binding object.  The same
   section also says `le ninmu goi la .sam.` assigns "Sam" only for the current
   context, not as the woman's ordinary name; referents now have
   `assignedNames` entries for that semantic fact.

48. Expanded assigned pro-bridi in bridi and restrictive contexts.
   CLL 7.5's `mi klama cei brode le zarci .i do brode` requires inherited
   argument places with explicit current places overriding them, while
   `le crino broda` after a long `cei broda` assignment requires the pro-bridi
   to behave like the assigned selbri inside a description/tanru.  JSON now
   emits the expanded predications directly, without public definition or
   binding objects and without leaving a bare unknown `broda` relation.

49. Resolved letteral pro-sumti by name/description initial.
   CLL 7.5 states that BY words in sumti position can refer to the latest name
   or description beginning with the same letter, as in `gy.` after `le gerku`.
   Resolved letteral pro-sumti now point directly at the antecedent referent;
   unresolved cases remain diagnostic rather than inventing an antecedent.

50. Expanded resolved GOhA pro-bridi directly.
   CLL 7.6 says `go'i` repeats the previous bridi, `go'e` the second previous
   bridi, and current explicit sumti override inherited places.  Leaving
   `go'i` as a one-place opaque relation lost the antecedent relation and place
   inheritance.  Resolved GOhA now lowers to the antecedent predication
   relation with inherited non-overridden places and inherited tense/space
   anchors unless the current pro-bridi supplies its own anchors.

51. Added quote-local anaphora streams.
   CLL 7.6 states that anaphora inside quotation cannot refer to the supporting
   text outside the quote, while related quotations can refer to one another.
   A single discourse-wide pro-bridi stream made quoted `go'i` select the
   surrounding `cusku` bridi.  Quoted text now resolves pro-bridi and pro-sumti
   against quote-local mention streams.

52. Added NU-description reification and recursive-copy elision.
   CLL 7.6's `le si'o mi go'i` requires the sumti to denote a concept, not an
   entity satisfying an opaque `si'o go'i` relation.  Descriptions headed by NU
   now produce referents sorted by abstraction output, link them to an
   `abstraction` object with relations such as `conceptOf`, `eventOf`, or
   `propertyOf`, and put the embedded formula in the abstraction body.  When a
   copied pro-bridi argument would contain the pro-bridi source itself, as in
   `le nu nei`, that inherited place is elided with a diagnostic so the public
   graph remains finite.

53. Scoped leading `xu` to the first statement.
   CLL 7.6 uses `xu ... .i go'i` where only the first bridi is the truth
   question and the following `go'i` answer is an assertion.  Treating leading
   `xu` as a text-wide force marker incorrectly made all subsequent statements
   questions.  Leading truth-question force is now consumed by the first actual
   statement in the text.

54. Added typical-place-value referents for `zu'i`.
   CLL 7.7 distinguishes `zu'i` from `zo'e`: `zo'e` is elliptical and can be
   omitted, while `zu'i` explicitly asks for the typical value of that
   particular place.  Treating `zu'i` as generic unresolved `proSumti` lost this
   contrast.  A `zu'i` argument is now `kind = "filled"` and points at a
   referent whose descriptor has `kind = "typicalPlaceValue"`.

55. Added predication-level reciprocity for `soi`.
   CLL 7.8 says `soi` asserts that the bridi remains true when two participants
   are interchanged, and that a single explicit participant uses the immediately
   preceding sumti as the other participant.  Dropping `soi`, treating it as a
   separate equality, or rebuilding its participants as fresh referents loses
   that predicate-specific interchange.  Predications now have `reciprocity`,
   whose entries contain structured `left` and `right` participant fillers and
   `introducedBy = "soi"`.  `vo'a`-series participants resolve to the current
   predication's x1-x5 fillers; omitted `soi` participants reuse the host
   sumti's existing filler.

56. Added relation-parameter predications and vocative-local questions.
   CLL 7.9's `mo` asks for a relation, not for an entity and not for a lexical
   relation named `mo`.  Predications can therefore use `relationParameter` to
   point at a `parameter` object with `sort = "relation"` and
   `role = "relationQuestion"`, while preserving the visible argument fillers
   that the answer relation must apply to.  The same section's `doi ma` asks
   who the vocative target is; the vocative utterance now contains a direct
   question whose body is a performative `vocativeTarget` predication instead
   of leaking the `ma` parameter into a following bridi or dropping the question
   altogether.

57. Added implicit existential scopes for bare da-series variables.
   CLL 7.12's `da poi grana ... gi'e ... da` uses one bound variable over both
   connected bridi tails.  Leaving bare `da` as an opaque `proSumti` referent
   preserved coreference but lost existential scope and the restrictive `poi`
   condition as a quantifier restriction.  Bare `da`, `de`, and `di` now wrap
   the host formula with `operator = "exists"`, `variable` pointing at the
   shared referent, and no `quantity`; any restrictive relative clause on the
   variable supplies the wrapper `restriction`.

58. Added definitional identity mode for `du`.
   CLL 7.14 says `du` is not a pro-bridi and is not the ordinary sameness
   predicate `mintu`; it is an identity sentence used to define or identify
   attached sumti as representations of the same referent.  Public JSON now
   emits `relation = "identity"` with `mode = "definitional"` for `du`, while
   `mintu` remains an ordinary asserted relation with its own x3 standard
   place.

59. Added rafsi binding metadata for pro-sumti lujvo components.
   CLL 7.15's `fo'arselsanga` uses a rafsi from assigned pro-sumti `fo'a`.
   Leaving the relation as a plain lexical label preserved the surface relation
   but lost the context-sensitive component that the section is demonstrating;
   desugaring the lujvo into asserted component relations would overstate the
   discourse truth.  `RelationExpansion.rafsiBindings` now records each such
   surface rafsi, its resolved source word, and the referent it resolves to
   when one is known.

60. Added semantic lowering for GOI relative phrases and optional
    `RelativeClause.introducedBy`.
    CLL 8.3 shows that `pe`, `po`, `po'e`, `po'u`, `ne`, and `no'u` are not
    implementation-only attachment markers; they contribute association,
    possession, or identity content to the head sumti.  Dropping
    `SumtiAssociationPhrase` preserved the base description but lost the
    restrictive/incidental relation that identifies examples such as
    `le stizu pe mi` and `le nanmu no'u la .djim.`.  Relative phrases now lower
    to ordinary formula bodies attached through `relativeClauses`, with
    `introducedBy` preserving the shortcut cmavo and the formula predication
    using `associatedWith`, `specificallyAssociatedWith`,
    `intrinsicallyPossessedBy`, or `identity`.

61. Added non-veridical restrictive relative clauses for `voi`.
    CLL 8.5 says `voi` is restrictive like `poi`, but non-veridical like a
    speaker description: `ti voi mlatu cu gerku` should not assert that `ti` is
    a cat.  Treating `voi` as incidental, or as a plain restrictive
    `mlatu(ti)` clause, loses this contrast.  `RelativeClause.veridical =
    false` now marks `voi`, and its body lowers to `describedAs(speaker, head,
    property)` where the property is a `ka`-style abstraction over the `voi`
    bridi.

62. Added descriptor-scoped `Descriptor.relativeClauses`.
    CLL 8.6 distinguishes relative clauses inside a description from clauses
    after the closed sumti, especially with outer quantification.  `lo prenu noi
    blabi` describes persons with an incidental description-internal clause,
    while `lo prenu ku noi blabi` qualifies the selected sumti occurrence.
    Putting both on `ArgumentValue.relativeClauses` made those scopes
    indistinguishable.  Description-internal `poi`/`noi`/GOI clauses now appear
    as `Descriptor.relativeClauses`; leading description-tail clauses such as
    `le poi blabi ku'o gerku`, post-`ku` clauses, and relative clauses on bare
    indefinite descriptions such as `re karce poi xekri` remain
    occurrence-scoped.

63. Added explicit possessive-sumti association.
    CLL 8.7 says `le mi karce` is semantically the same weak association as
    `le pe mi karce`, not merely a structural operand.  `Descriptor.operand`
    still points at the possessor referent, but the descriptor now also carries a
    restrictive `associatedWith(head, possessor)` relative clause.  If a relative
    clause immediately follows the possessor, as in `le mi noi sipna vau karce`,
    that clause qualifies the possessor argument of `associatedWith`, not the
    possessed referent.

64. Added predication-level place-question bindings for `fi'a`.
    CLL 9.3 says `fi'a` asks which FA place tag would make the bridi true, and
    that the `fi'a` term does not itself become a numeric FA assignment.  In
    `fi'a do dunda fe le vi rozgu`, the addressee is not simply x1; the
    question is whether the addressee fills x1 or x3, while x2 is already the
    rose.  `parameter.sort = "place"` and `role = "placeQuestion"` now model
    the answer slot, and predications use `placeQuestions` to bind that slot to
    the known argument and candidate numbered places.  This was not expressible
    with only the `arguments` map, because the questioned value is a place
    label rather than an entity.

65. Clarified duplicate FA lowering as conjoined claims.
    CLL 9.3 says repeated explicit FA tags make multiple claims, e.g. repeated
    x1 and x2 fillers for `klama` distribute into all relevant goer/destination
    combinations.  No new object type is needed: the existing formula
    conjunction shape is used with one atom per concrete predication, and
    unmentioned places remain shared bridi-level elisions.

66. Changed modal arguments from one filler to a place map. **Refined by
    amendment 40 and issue #126 for `fi'o`.**
    CLL 9.5 says `fi'o kanla le zunle` makes the modal sumti fill x1 of
    `kanla`, and `fi'o se pilno le zunle kanla` makes it fill x2 of `pilno`.
    A single `argument` field could preserve the modal sumti but not the tag
    relation's place structure, conversion, or omitted non-x1 places.  Modal
    entries were extended to use `arguments`, the same numbered argument map as
    predications.  Current normative form keeps that map for fixed lexical BAI
    tags; ad-hoc `fi'o` entries use a full modal `body` formula so even a
    one-brivla tag such as `fi'o kanla` is represented as a subordinate
    predication body with explicit places, not as a relation-string shorthand.

67. Use source relations for BAI modal tags.
    CLL 9.6 defines BAI tags through the place structures of corresponding
    gismu: `pi'o` is based on `pilno`, `ka'a` on `klama`, and `ga'a` on
    `zgana`.  Emitting `relation = "ka'a"` made the public graph depend on an
    obscure cmavo as if it were a predicate, and it prevented known omitted
    places such as `klama` x2-x5 from being explicit.  BAI modal entries now
    keep the cmavo in `introducedBy` but use the source relation in `relation`;
    SE conversion selects which source-relation place the visible modal sumti
    fills.  The special vague modal `do'e` remains `unspecified-modal`.

68. Added sequence `connectionClaims`.
    CLL 9.7 says `.iri'abo` and `.iseri'abo` assert both connected bridi and
    also assert a causal relation between the events described by them.  A
    plain sequence could preserve the two utterances but had no public place
    for the extra `rinka` claim; making the sequence itself truth-valued would
    blur discourse organization with formula structure.  `connectionClaims`
    records formula IDs for such connective-introduced claims while leaving
    `items` restricted to utterances or nested sequences.

69. Allowed formula IDs as proposition-like argument fillers.
    CLL 9.8's `li ny. du li vo .ini'ibo li ny. du li re su'i re` uses `ni'i`
    from `nibli`, a relation whose x1/x2 are propositions.  The earlier
    event-only modal-connection lowering worked for `rinka` and `mukti`, but it
    could not represent logical entailment between identity statements without
    pretending that identities are ordinary events.  `ArgumentValue.filled`
    may now point at a `formula` when the relation's place structure calls for
    a proposition-like argument.

70. Interned mathematical variable number referents by lerfu-string name.
    CLL 17.11 treats lerfu strings in mekso as mathematical variables.  In CLL
    9.8, both occurrences of `li ny.` in the two identity statements are the
    same variable `n`; emitting two unrelated number referents obscured the
    intended entailment.  Repeated `li` sumti whose mekso is a lerfu-string
    variable now reuse one number referent within the discourse.  Arithmetic
    expressions and numeric constants are not normalized by this rule.

71. Clarified modal selbri and text-group modal scope.
    CLL 9.9 allows a BAI or `fi'o` modal to appear before a selbri without an
    explicit modal sumti, and also allows one modal to scope over a connected
    bridi-tail or `tu'e` group.  No new object type is needed: these use the
    existing `modalArguments` place-map shape.  Bare modal selbri create
    source-relation place maps with elided fillers; spread modals duplicate the
    modal entry on each asserted predication while reusing the same filler IDs
    for the single modal's elided participants.

72. Clarified explicit places for converted property modifiers.
    CLL 9.9 compares `bai tavla` to `se bapli tavla`, which exercises the
    uniform tanru/property lowering with the property slot in a converted
    non-x1 place.  The all-places-explicit rule applies there as well:
    restrictive property predications must include every known place of the
    source relation, including places before the filled converted place.

73. Added source-relation lowering for modal relative phrases.
    CLL 9.10 says `pe cu'u` and `ne fi'e` have the full semantic content of
    the corresponding `poi se cusku` and `noi se finti` clauses, and that
    `ne semau`/`ne seme'a` use the comparative source relations `zmadu` and
    `mleca`.  Plain `pe`/`ne` can only say `associatedWith`, but modal relative
    phrases with known CLL routing must not lose the source relation.  No new
    object type is needed: the existing relative-clause body points at a
    source-relation predication with `introducedBy` preserving the modal marker.

74. Added content-described eventualities for grouped modal connection sides.
    CLL 9.11 says the `tu'e`/`ke` grouped portion of examples 9.79-9.81 is the
    effect as a whole: carrying the dog or carrying the cat, equally.  Picking
    the first branch's event made `se ri'a` point only at carrying the dog, and
    a discourse `sequence` cannot itself fill an event place.  Eventive modal
    connection claims may now fill a source-relation event place with an
    `eventuality` whose `content` points at the grouped `formula` or nested
    `sequence`; atomic branches still use their own predication eventualities.
    The same audit also requires nested logical sumti groups such as
    `le gerku .adu'ibo le mlatu` under outer `.eseri'ake` to lower recursively
    as formula children, not as a fabricated referent.

75. Added `Sequence.content` for explicit statement logical connections.
    CLL 9.11 says `.ije` and mixed `.ijeki'ubo` assert the logical connection
    between the two statements, not merely two adjacent utterances.  Making the
    root a bare `formula` would lose the two utterance acts, while leaving a
    plain sequence with a diagnostic lost the truth-functional claim.  A
    sequence may now carry `content` pointing at the statement-level connective
    formula; modal side claims such as `ki'u`, `ri'a`, and `du'i` remain in
    `connectionClaims`.

76. Clarified `jai BAI` modal conversion as modal-argument routing.
    CLL 9.12 says `la .lojban. jai bau cusku fai mi` is equivalent to
    `mi cusku bau la .lojban.`: the language place introduced by `bau` becomes
    visible as the converted x1, while `fai` restores the old x1 of `cusku`.
    The previous JSON model had `modalArguments` and source-relation routing
    for ordinary BAI tags, but did not say how a converted modal place should
    appear.  No new object or field is required; the affected predication keeps
    the inner relation and records the raised place as the same BAI
    `modalArguments` entry that an ordinary modal sumti would have produced.

77. Added modal-argument polarity fields.
    CLL 9.13 distinguishes contradictory negation on a modal (`mu'inai`) from
    scalar negation before a modal (`na'emu'i`).  The old JSON could preserve
    the source text but had no machine-readable way to say that the host claim
    was asserted while only the modal relation was negated.  `ModalArgument`
    now has optional `negation` and `scalarNegation` fields.  `negation` is an
    embedded object with `kind = "contradictory"` and `introducedBy = "nai"`;
    `scalarNegation` reuses the existing scalar-negation shape, for example
    `{"kind":"otherThan","introducedBy":"na'e"}`.

78. Clarified sticky modal lowering.
    CLL 9.14 says a BAI modal followed by `ki` persists, together with its
    following sumti, into following bridi until cancelled.  The JSON graph does
    not need a public sticky-binding object: the resolved modal argument is
    repeated on affected asserted predications.  This was necessary for
    `mi tavla ... bai ki tu'a la .frank. .ibabo mi tavla ...`, where the second
    `tavla` must carry the same `bapli` modal argument pointing at the original
    `tu'a la .frank.` referent.  Bare `ki`, as in `mi ki tavla`, clears sticky
    context and emits no modal relation.

79. Clarified logical modal-tag connections.
    CLL 9.15 says `la .frank. bajra seka'a je teka'a le zdani` is equivalent
    to the statement connection in example 9.91: Frank runs to the house and
    Frank runs from the house, without implying whether those are one or two
    acts of running.  Treating `seka'a je teka'a` as a single opaque modal
    relation lost both the logical connective and the `klama` source-relation
    place routing.  No new object type is required: the utterance content is a
    modal-locus connective formula with one branch predication carrying the
    `se ka'a` modal argument and the other carrying the `te ka'a` modal
    argument.  By contrast, the `ce'e` termset in example 9.93 remains one
    host predication with both modal arguments, because CLL says the termset
    conventionally forces one common running event.

80. Clarified moved tense terms as event anchors.
    CLL 10.1 says `mi cu pu klama le zarci`, `puku mi klama le zarci`,
    `mi klama puku le zarci`, and `mi klama le zarci pu` differ only in
    emphasis.  The previous JSON lowering treated moved `puku` as a modal
    argument relation named `pu` with an elided place, which confused tense
    anchoring with BAI-style modal arguments.  No new field is required:
    moved tense terms attach the same `time` or `space` anchor to the host
    predication's eventuality that selbri-adjacent tense would attach.

81. Added ordered `Eventuality.recurrence`.
    CLL 10.10 gives `re'u` as ordinal tense and contrasts
    `mi pare'u paroi klama le zarci` with
    `mi paroi pare'u klama le zarci`; v0 lowered at least one of those forms
    in a way that did not expose both recurrence layers.  A flat aspect label
    or unordered frequency field cannot represent the contrast.  Eventualities
    now carry an ordered `recurrence` list whose entries record the recurrence
    kind, source marker, and optional numeric value.  ZAhO event contours remain
    in `aspect.contour`, using CLL-derived English labels such as
    `prospective`, `initiative`, and `superfective`.

82. Added interval and FEhE spatial event fields.
    CLL 10.11 says `fe'e` transfers interval properties and ZAhO contours to
    space: `ko vi'i fe'e di'i sombo le gurni` is evenly distributed along a
    line, and `tu ve'abe'a fe'e co'a rokci` uses the beginning contour for the
    south face of a northward spatial interval.  Putting those forms in
    ordinary `recurrence` or `aspect` incorrectly makes them temporal/event
    contour claims.  Eventualities now distinguish `timeInterval`,
    `spaceInterval`, `spatialRecurrence`, and `spatialAspect`; `fe'e` routes
    the following interval property or ZAhO contour to the spatial fields.

83. Added anchors for tense sumtcita event attributes.
    CLL 10.12 says tense tags before sumti are relational: `ca le nu ...`
    means simultaneous with that event, `vi le panka` means near the park, and
    `ba'o le nu ...` means in the aftermath of that process.  The previous
    v1 lowering treated these as moved tense terms, anchoring to speech time or
    current location and dropping the following sumti.  Event attributes that
    can be used as sumtcita now carry optional anchors: `time.anchor` and
    `space.anchor` already point at the tagged sumti, while `aspect.anchor`,
    `spatialAspect.anchor`, `timeInterval.anchor`, `spaceInterval.anchor`, and
    `recurrence[].interval` cover ZAhO, FEhE-ZAhO, ZEhA/VEhA, and ROI/TAhE
    sumtcita.

84. Added ordered `Eventuality.timePath`.
    CLL 10.13 says multiple tense constructs are cumulative and order-sensitive:
    `puba` means past-then-future, while `bapu` means future-then-past.  A
    single `time` relation loses that contrast, and duplicating contradictory
    before/after claims does not say which one is interpreted from which
    reference point.  Multi-step temporal journeys now use `timePath`, whose
    first step has an object anchor such as `eventuality:3` and whose
    later unanchored steps use `{"kind":"previous"}`.  Single-step temporal
    direction still uses the existing compact `time` field.

85. Clarified subordinate tense anchoring.
    CLL 10.13 also says tenses in subordinate descriptions and abstractions are
    interpreted relative to the main bridi's tense.  Without an explicit anchor,
    `le ba'o zarci` in `mi pu klama le ba'o zarci` could be misread as
    retrospective relative to speech time rather than relative to the going
    event.  No new field is required: subordinate `time`, `aspect`,
    `timeInterval`, `recurrence[].interval`, and compound `timePath` object
    anchors may point at the containing predication's eventuality.

86. Added temporal direction distance on `time` and `timePath`.
    CLL 10.14 uses `pu zu` for a long time before the reference time and
    `ba za` for a medium time after it.  The previous model had interval
    extents for `ze'i/ze'a/ze'u/ze'e`, but no place for `zi/za/zu` when they
    modify a temporal direction.  `time.distance` and
    `timePath[].distance` now record `short`, `medium`, or `long` on the
    affected direction itself, preserving the distinction between "long before"
    and "before during a long interval".

87. Added ordered `Eventuality.spacePath`.
    CLL 10.3 says compound spatial tenses are imaginary journeys whose order is
    meaningful: `ga'u zu'a` and `zu'a ga'u` unfold differently.  A single
    `space` relation loses that order, just as a single `time` relation loses
    the `puba`/`bapu` contrast.  Multi-step spatial journeys now use
    `spacePath`, with the same object/previous anchor shape as `timePath`.
    This also lets `ca'u vi ni'a va ri'u vu ne'i` record each FAhA step and its
    associated VA distance without fabricating intermediate referents.

88. Clarified contextual story-time lowering.
    CLL 10.14 says story time is a discourse convention: the same surface
    syntax can be interpreted with ordinary sticky tense or with narrative
    advancement, depending on context.  The object graph therefore does not
    guess story time from text shape.  When the caller explicitly selects the
    story-time context, the existing `time` and `timePath` anchors point at
    prior story-event eventualities so that CLL 10.14's order
    `10.93 - 10.95 - 10.94 - 10.96 - 10.97 - 10.98 - 10.99` is represented
    directly.

89. Added text descriptions for `se du'u`.
    CLL 11.7 says `du'u` x1 is the predication/proposition and x2 is a
    sentence expressing it, and CLL 10.15 relies on `le se du'u` for the text
    expressed by `cusku`.  Treating `le se du'u` like bare `le du'u` loses that
    x1/x2 distinction and fills text places with proposition referents.  The
    described referent for `se du'u` is now sort `text`, and its descriptor body
    uses the relation `sentenceExpresses(text, abstraction)`.

90. Clarified tense connection claims and inert branch descriptions.
    CLL 10.16 says afterthought and forethought tense sentence connectives
    claim both connected sentences and the tense/space relation between their
    events.  It also says the parallel sumti and bridi-tail forms do not claim
    their underlying sentences; only the relation is claimed.  The model already
    had `connectionClaims` and `mode = "inert"`, so no new object or field was
    required.  The amendment clarifies that `pu`/FAhA tense connectives produce
    asserted relation predications such as `before` or `leftOf`, and that pure
    tense relation-only sumti/bridi-tail forms keep branch predications inert
    while using their eventualities as arguments of the asserted relation.

91. Clarified grouped tensed logical connectives.
    CLL 10.17 says `.ije ba tu'e ... tu'u`, `gi'e bake ... ke'e`, and
    `.e bake ... ke'e` are grouped versions of the same tensed logical
    connective pattern.  A text-group tense should not become a generic modal
    argument with an elided anchor on every nested predicate.  Instead the
    sequence that holds the grouped right side is reified as an eventuality and
    related to the left side by an asserted connection claim.  No new field was
    required; this reuses `connectionClaims` plus `eventuality.content`.

92. Added scalar negation on event relations and aspects.
    CLL 10.18 distinguishes contradictory tense `nai` from scalar `NAhE`
    before tense constructs.  Contradictory `punai`, `ne'inai`, and `ca'onai`
    are represented by formula-level `operator = "not"` around the positive
    tensed atom, because the bridi as a whole is false and the model must not
    also negate the event relation internally.  Scalar `na'e pu`, `to'e ne'i`,
    `na'e ca`, and `na'e ca'o` assert the host predication while modifying the
    relevant event attribute, so `time`, `timePath[]`, `space`, `spacePath[]`,
    `aspect`, and `spatialAspect` now have optional `scalarNegation`.

93. Clarified explicit CAhA actuality.
    CLL 10.19 says a bridi without CAhA may describe an actual event, a
    capability, or a potential event.  The earlier JSON builder defaulted every
    predication eventuality to `actuality.kind = "actual"`, which made
    `ta jelca` and `ro datka ca flulimna` indistinguishable from explicit
    `ca'a`.  The model now treats omitted CAhA as unspecified by omitting
    `actuality`; explicit `ca'a`, `ka'e`, `nu'o`, and `pu'i` fill
    `actuality.kind` with `actual`, `capable`, `potential`, and
    `demonstrated`.

94. Added distance-only temporal relations for bare `ZI`.
    CLL 10.4 says `zi` by itself signals an event close to the present without
    saying whether it is past or future, and likewise `zu` signals a remote
    time with unspecified direction.  The previous JSON builder only used
    `ZI` as the distance on a preceding `PU`, so `zu` was dropped and `zi pu`
    lost the first path step.  `zi`, `za`, and `zu` now emit temporal
    relations `near`, `mediumDistance`, and `far` when they do not modify a
    preceding `PU`; compounds such as `zi pu` use `timePath` to preserve the
    ordered imaginary journey.

95. Added spatial `motion` on event spatial relations and path steps.
    CLL 10.8 distinguishes static `ri'u` from motion `mo'i ri'u`, and combines
    static and moving spatial steps in examples such as `zu'avu mo'i ri'uvi`.
    A single event-level location field could not preserve which step is
    movement.  `space` and `spacePath[]` now have optional `motion`, currently
    with `kind = "toward"` and `introducedBy = "mo'i"`, attached only to the
    relation scoped by `mo'i`.

96. Added recurrence-level `negation`.
    CLL 10.9 gives `ru'inai` as intermittent and `reroinai` as other than
    twice.  Formula-level tense negation would be too broad here, while
    changing only the recurrence kind would lose the counted value for ROI.
    `recurrence[]` and `spatialRecurrence[]` now have optional `negation`, so
    `ru'inai` and `reroinai` preserve the base recurrence marker and record the
    following `nai` directly on that recurrence.  This recurrence-level `nai`
    does not also wrap the host formula in `operator = "not"`.

97. Clarified `ma'i` as a source-relation modal argument.
    CLL 10.8 uses `ma'i vo'a` to change the reference frame for spatial
    direction.  This did not require a new public object: ordinary BAI modal
    arguments already attach a relation to the host predication.  The
    important correction is that `relation` should be the source relation
    `manri`, not the modal marker `ma'i`; `introducedBy` still preserves the
    source marker.

98. Added ordered aspect chains.
    CLL 10.21 allows strings of event contours, as in `ca'o co'a`, and the
    earlier scalar `aspect` field overwrote one contour with the other.  A
    single contour still uses `aspect`; multiple contours use ordered
    `aspects`, with the scalar field omitted.  Spatial contour chains use
    `spatialAspects` analogously.

99. Added recurrence product connections.
    CLL 10.21 uses `reroi pi'u xaroi` for a cross-product of recurrence sets:
    two occasions, each containing six shots.  A flat recurrence list preserved
    both counts but not the product relation between them.  Recurrence entries
    now have optional `connection`; the second entry in this example carries
    `{"kind":"product","introducedBy":"pi'u"}`.

100. Added bounded temporal `timeSpan`.
    CLL 10.20 uses `puza bi'o bazu` for an event spanning from a medium time
    before the reference point to a long time after it.  `timePath` was the
    wrong shape because it says to move before the anchor and then after that
    derived point; `timeInterval` was also insufficient because it records
    ZEhA extents, not BIhI endpoints.  Eventualities now have optional
    `timeSpan` with `start`, `end`, and `introducedBy`, where each endpoint
    records the temporal relation, anchor, source marker, and optional
    distance.

101. Added tense-modal and connective question slots.
    CLL 10.24 distinguishes `ca ma`/`vi ma`, where `ma` is an ordinary entity
    parameter used as a time or place anchor, from `cu'e`, which asks for the
    tense or modal construct itself.  Eventualities now have optional
    `tenseModal`, pointing at a `parameter` with `sort = "tenseModal"` and
    `role = "tenseQuestion"`.  The same section also uses `je'i` to ask which
    logical connective relates two tense branches.  Formula connectors now have
    optional `parameter`, and such formulas use
    `operator = "connectiveQuestion"` with a `parameter` of
    `sort = "connective"` and `role = "connectiveQuestion"`.  Tense/modal
    fragment answers are represented as mentioned eventualities carrying the
    answer's `time`, `space`, `aspect`, or `modalArguments`.

102. Added exact event-relation `magnitude`.
    CLL 10.25 uses `zu'a nu'i ... la'u ...` to specify an origin and an exact
    distance for a spatial tense/modal tag.  The previous model had vague
    `distance` for VA/ZI markers, but no field for an exact amount supplied by
    a termset sumti.  `time`, `timePath[]`, `space`, and `spacePath[]` relation
    objects now have optional `magnitude`, whose `value` points at the supplied
    magnitude referent and whose `introducedBy` records markers such as
    `la'u`.  Governed termset branches are not ordinary predication arguments.

103. Clarified propositional-attitude assertion effect.
     CLL 10.26 begins with `.a'o`, and CLL 13.3 says propositional attitudes
     subordinate the host proposition rather than simply adding an independent
     asserted claim.  The preexisting `displayedContent` object and
     `assertionEffect` field now have a concrete rule for these indicators:
     the display targets the host formula, anchors to the utterance, and uses
     `assertionEffect = "hostSubordinated"`.  No separate truth-bearing
     predicate is added for the attitude.

104. Lowered NU-as-selbri through abstraction links. **Superseded by
     amendment 27's direct-output rule.**
     CLL 11.1 says abstraction selbri have ordinary selbri uses, while CLL 11.2
     gives `nu` the place structure "x1 is an event of (the bridi)".  The
     previous model text allowed `nu ...` to appear as a string relation label
     in tanru contexts, which lost the first-class abstraction and its embedded
     formula.  NU used directly as a selbri now emits the same constructed
     relation used by NU descriptions, e.g. `eventOf(x1, abstraction)`, with
     the embedded bridi inert in the abstraction body.  NU used as a tanru
     seltau produces a property over the abstraction output sort, so
     `nu sonci kei djica` has an eventuality-sorted `propertySlot` parameter
     and does not assert either `nu sonci(djan)` or a concrete soldier-event
     referent for John.

     Current normative form: the NU output object itself carries the body; a
     predication shape is used only where the grammar needs a selbri, and it
     points at the direct output object rather than at an `abstraction` wrapper.

105. Split event-type abstraction links and exposed extra places. **Superseded
     by amendment 27's direct-output rule.**
     CLL 11.3 gives separate place structures for `mu'e`, `pu'u`, `zu'o`, and
     `za'i`.  A single `eventOf` relation hid those distinctions, and it had no
     room for the x2 stages/actions places of `pu'u` and `zu'o`.  The link
     relations were made type-specific: `achievementOf`, `processOf`,
     `activityOf`, and `stateOf`.  That historical constructed-link shape used
     x2 for the embedded abstraction object and shifted additional surface
     places after it, so the elided stages/actions place appeared as x3 on
     `processOf` and `activityOf`.
     Current normative form: the output eventuality referent carries the
     specific sort (`eventuality/achievement`, `eventuality/process`,
     `eventuality/activity`, or `eventuality/state`) and any real extra surface
     place directly.

106. Added implicit `ka` property slots.
     CLL 11.4 states that a property abstraction without explicit `ce'u` places
     uses the first unfilled surface place as the focus, and contrasts
     `ka mi prami` with `ka prami mi`.  The previous model only counted
     explicit `ce'u`, which made these examples arity 0 and lost the fillable
     position.  Such abstractions now create a normal `propertySlot` parameter
     with `introducedBy = "implicit ce'u"` and place it directly in the body
     formula.  Converted selbri use visible place order, so `ka se risna`
     places the parameter in raw `risna` x2.

107. Added math-expression leaves for `mo'e` sumti operands.
     CLL 11.5 uses `li pa vu'u mo'e le ni le pixra cu blanu` for `1 - B`,
     where `B` is the amount described by the `ni` abstraction.  The previous
     output represented the right operand as an opaque math literal, losing the
     amount output object and its abstraction body.  A `mo'e` operand
     now emits a `mathExpression` leaf with `literal.kind = "sumtiOperand"` and
     `denotes` pointing directly at the full sumti denotation.  If that
     denotation is formula-scoped by an outer quantifier, see amendment 36.

108. Exposed minor-abstraction x2 places. **Revised by amendment 27.**
     CLL 11.13 gives `li'i` and `si'o` their own x2 places: experiencer and
     mind respectively; it also gives `ni`, `jei`, and `du'u` x2 places
     elsewhere in the abstraction family.  The older constructed-link shape
     shifted those places to x3 because x2 was occupied by the first-class
     `abstraction` wrapper.  Under the direct-output rule, these are fields on
     the output object itself (`experiencer`, `mind`, `scale`, `epistemology`,
     `expressedBy`).  CLL 11.13 gives no x2 for `su'u`; do not fabricate one.

109. Clarified `tu'a` and CLL 11.10 bare/BAI `jai` raising.
     CLL 11.10 says `tu'a` and bare `jai` both raise an argument to stand for
     an implicit abstraction involving that argument, but only `jai BAI`
     identifies a specific source-relation place.  The existing
     `abstractionAbout` descriptor was sufficient for explicit `tu'a`, but the
     model text did not say how bare `jai` should be represented.  Bare `jai`
     now uses the same descriptor shape with `word = "jai"`; `jai gau` and
     other `jai BAI` forms instead use `modalArguments` with the BAI source
     relation (`gasnu` for `gau`) and keep the inner predicate's old place
     structure explicit.

110. Added connected abstractor formulas.
     CLL 11.12 says `le pu'u jenai za'i mi sipna` describes the process of
     sleeping but not the state of sleeping.  A single abstraction link could
     only emit `processOf` and dropped the negated `za'i` branch.  Connected
     abstractors now build one type-specific abstraction link per abstractor
     and combine those links with the normal formula connective shape, using
     `connector.locus = "abstraction"` and `not` for `nai`-negated branches.

111. Added displayed-content modifiers and attitude-question displays.
     CLL 13.7 and 13.8 use UI words such as `se'i`, `ro'o`, and `dai` as
     modifiers of a preceding attitudinal rather than as independent displayed
     content.  The previous displayed-content object had no place to keep
     those modifiers, so a public graph either dropped them or misrepresented
     them as separate attitudes.  Displayed content now has `modifiers`, each
     with a relation and optional polarity/intensity; `dai` shifts the
     `experiencer` to an elided empathetic experiencer.  CLL 13.10 uses `pei`
     to ask for an attitude, so `pei`-marked displays use
     `family = "questionPrompt"` with relations such as `agreementQuestion`.

112. Allowed displayed-content sequences as mention content.
     CLL 13.10 has indicator-only utterances with multiple independent
     attitudes, such as `.iu bu'onai .uinai`.  A single displayed-content object
     could not preserve both displays, and an utterance can only point at one
     `content` object.  A `sequence` may now group displayed-content items when
     the sequence is used as non-truth-valued mention content.

113. Added composition exclusions for sumti connective `nai`.
     CLL 13.8 contrasts `mi .e .ui nai do` with `mi .e nai .ui do`; the second
     requires the "not you" side to be public semantic content.  A composite
     referent with only `members` could not distinguish `mi .e do` from
     `mi .e nai do`.  `composition.excludedMembers` now records referents
     excluded by connective `nai`.

114. Added standalone `attitudeModifier` displays.
     CLL 13.7 says modifier UI words such as `ga'i` and `be'u` can be used
     alone, targeting the marked referent or bridi.  Treating them as raw
     metalinguistic cmavo lost their English semantic relation.  Standalone
     modifier words now emit displayed content with
     `family = "attitudeModifier"` and translated relations such as `rank` and
     `need`; when they follow a base attitudinal, they remain entries in that
     display's `modifiers` array.

115. Clarified displayed-content targets for indicator-only discourse acts.
     CLL 13.11 has `ru'a doi .livinston.`, where the evidential scopes over a
     vocative-only utterance, and CLL 13.3 has `do sazri ... .i .e'a`, where an
     indicator on a bare separator comments on the previous claim.  The prior
     model text described formula and referent targets, but did not say what to
     do when the marked discourse has no new formula.  Displayed content may
     target the utterance itself for vocative-only text, or the previous
     utterance's content when the indicator is on a separator with no following
     statement.

116. Added scalar-negated non-logical composition.
     CLL 14.15 states that `nai` after JOI does not negate either connected
     sumti, but says that the named connection is inapplicable and some other
     connection is intended.  The previous `excludedMembers` field was correct
     for logical `mi .e nai do`, but wrong for `mi jo'u nai do` because it made
     the addressee look semantically negated.  `composition.scalarNegated`
     records this JOI-specific scalar negation while preserving both members.

117. Added interval composition complement and endpoint inclusion.
     CLL 14.16 gives `bi'i`, `bi'o`, and `mi'i` interval connectives, GAhO
     endpoint inclusivity, and BIhI `nai` interval complements.  A plain
     `operator` plus `members` could not distinguish inclusive/exclusive
     endpoints or `bi'i nai` from right-branch exclusion.  Interval
     compositions now use `unorderedInterval`, `orderedInterval`, or
     `centeredInterval`; explicit GAhO markers become `endpointInclusion`, and
     BIhI `nai` becomes `complement: true`.

118. Clarified direct `xu` consumption by question objects.
     CLL 14.13 says leading `xu` prefixes a statement to make a truth question.
     The JSON model already represents this as a direct `question` object whose
     `body` is the underlying formula, but the displayed-content text did not
     say that this `xu` is consumed by utterance force rather than emitted again
     as a question-prompt display.  A direct truth question has no fillable
     slot and no duplicate displayed-content prompt; other leading indicators
     on the same utterance target the question body.

119. Added connective signs for standalone connective answers.
     CLL 14.13 says bare logical connectives such as `gi'e nai` can answer a
     connective question by filling the connective blank, and CLL 14.15 extends
     the same response pattern to JOI answers such as `joi`.  A fixture that
     contains only the answer has no local question body to fill, so a missing
     formula diagnostic was too lossy.  Standalone connective answers now
     produce a mention utterance whose content is a `sign` with
     `kind = "connective"` and `text` set to the connective expression.

120. Added `composition.operatorParameter` for sumti connective questions.
     CLL 14.13 uses `ji` and forethought `ge'i` to ask which connective relates
     two sumti, as in `do djica tu'a loi ckafi ji loi tcati`.  Formula
     connective questions already had `connector.parameter`, but composite
     referents only had a string `operator`, so the sumti-level question was
     either lost or mislabeled as an ordinary `joint` composition.
     `operator = "connectiveQuestion"` plus `operatorParameter` now records the
     fillable connective slot inside the composite referent.

121. Clarified text groups as sequence content and forward `di'e` targets.
     CLL 14.15 uses `la'e di'e` to refer to a following `tu'e ... tu'u` text
     group.  The previous builder could resolve the syntax reference, but the
     target utterance did not exist yet during nested text validation.  Text
     groups now reserve their parenthetical utterance before building nested
     content, and the content is always a sequence, even for a one-utterance
     group.  This gives forward utterance pro-sumti a concrete target without
     making a parenthetical utterance contain another utterance directly.

122. Added `modalArguments[].modifiers` for indicators on modal tags.
     CLL 15.10 uses `go'i ji'una'iku` to mark the presupposition supplied by
     `ji'u` as metalinguistically wrong.  Treating `na'i` as a display on the
     whole host formula would lose the fact that the error is specifically in
     the modal assumption, while dropping it from `modalArguments` would lose
     the metalinguistic force entirely.  Modal arguments now carry nested
     displayed-content modifiers for indicators attached to the tag word.  The
     modifier still carries `assertionEffect:"metalinguisticallyVoided"`, because
     CLL 15.10 says `na'i` anywhere in the sentence makes it a non-assertion;
     the modifier location records that the presupposition/modal assumption is
     the offending part.

123. Added relation-variable parameters for `bu'a`-series selbri variables.
     CLL 16.13 uses `su'o bu'a zo'u la .djim. bu'a la .djan.` and
     `ro bu'a zo'u ...` to quantify over relationships, not over entity
     referents and not over a lexical relation named `bu'a`.  The previous
     model allowed `relationParameter` only for relation questions such as
     `mo`, and quantified formulas required `variable` to be a referent.  That
     made the prenex quantifier disappear or forced an incorrect
     `relation = "bu'a"` predication:

     ```json
     {
       "predication:1048": {
         "type": "predication",
         "relation": "bu'a",
         "arguments": {
           "x1": { "kind": "filled", "value": "entity:1049" },
           "x2": { "kind": "filled", "value": "entity:1050" }
         }
       }
     }
     ```

     The graph now uses a relation-sort parameter with
     `role = "relationVariable"`, puts it in the predication's
     `relationParameter`, and binds that same parameter with the surrounding
     quantifier formula:

     ```json
     {
       "parameter:1047": {
         "type": "parameter",
         "sort": "relation",
         "role": "relationVariable",
         "introducedBy": "bu'a"
       },
       "predication:1048": {
         "type": "predication",
         "relationParameter": "parameter:1047",
         "arguments": {
           "x1": { "kind": "filled", "value": "entity:1049" },
           "x2": { "kind": "filled", "value": "entity:1050" }
         }
       },
       "formula:1051": {
         "type": "formula",
         "operator": "forall",
         "variable": "parameter:1047",
         "body": "formula:1052",
         "quantity": "quantity:1152"
       }
     }
     ```

124. Clarified `naku` negation boundaries as formula-level `not`.
     CLL 16.9 and 16.11 distinguish prenex `naku` and in-bridi `naku`, but
     both create contradictory negation boundaries whose position matters.  The
     earlier builder could preserve prenex `naku` but dropped in-bridi `naku`
     terms such as `su'o verba naku klama su'o ckule`.  Public JSON now emits
     an explicit `operator = "not"` formula for each `naku`, with source
     construct `prenex-negation` or `bridi-negation-boundary` identifying where
     the boundary was introduced.  Adjacent `naku naku` therefore remains two
     nested `not` formulas rather than disappearing from the graph.

125. Added `sign.letterals` for letteral signs and multi-initial pro-sumti.
     CLL chapter 17 uses lerfu strings both as signs and as pro-sumti.  The
     earlier model had `kind = "letteral"` but no public payload for the
     letteral source, and the implementation treated standalone spelling
     examples as unresolved pro-sumti.  CLL 17.2, 17.3, 17.5, 17.6, 17.10, and
     17.13 require preserving `BU`, shifts, `tei`...`foi`, and `se'e` code
     forms; CLL 17.9 also requires longer pro-sumti such as `symydy.` and
     `.abupyky.` to resolve by multi-word name initials.

     Before, spelling `tanru` collapsed to a diagnostic referent:

     ```json
     {
       "entity:1011": {
         "type": "referent",
         "descriptor": { "kind": "unloweredSumti", "word": "sumti" },
         "diagnostics": [
           { "message": "letteral pro-sumti did not resolve to an antecedent" }
         ]
       }
     }
     ```

     Now mention contexts emit a letteral sign:

     ```json
     {
       "sign:1153": {
         "type": "sign",
         "kind": "letteral",
         "text": "tanru",
         "letterals": [
           { "kind": "glyph", "sourceWords": ["ty"], "value": "t" },
           { "kind": "glyph", "sourceWords": ["a", "bu"], "value": "a", "buDepth": 1 },
           { "kind": "glyph", "sourceWords": ["ny"], "value": "n" },
           { "kind": "glyph", "sourceWords": ["ry"], "value": "r" },
           { "kind": "glyph", "sourceWords": ["u", "bu"], "value": "u", "buDepth": 1 }
         ]
       }
     }
     ```

126. Added math-operator question parameters for `na'u mo`.
     CLL 18.19 uses `li re na'u mo re du li vo` to ask which mathematical
     operator makes `2 ? 2 = 4` true.  Treating `mo` as the literal operator
     string `"mo"` turns the sentence into an assertion and leaves no fillable
     question slot:

     ```json
     {
       "math:1124": {
         "type": "mathExpression",
         "operator": "mo",
         "operands": ["math:1123", "math:1122"]
       }
     }
     ```

     The graph now uses a `mathOperator`-sorted parameter with
     `role = "mathOperatorQuestion"`, places it in `operatorParameter`, and
     makes the utterance content a direct question:

     ```json
     {
       "parameter:1125": {
         "type": "parameter",
         "sort": "mathOperator",
         "role": "mathOperatorQuestion",
         "introducedBy": "mo"
       },
       "math:1124": {
         "type": "mathExpression",
         "operatorParameter": "parameter:1125",
         "operands": ["math:1123", "math:1122"]
       },
       "question:1146": {
         "type": "question",
         "kind": "mathOperator",
         "domain": "mathOperator",
         "body": "formula:1074",
         "slots": [{ "parameter": "parameter:1125", "role": "answer" }]
       }
     }
     ```

127. Added interval endpoint inclusion on math expressions.
     CLL 18.17 uses `ga'o` and `ke'i` around mekso interval connectives.  A
     generic math literal such as `{ "literal": { "kind": "expression",
     "value": "mekso" } }` lost the ordered/unordered interval relation and
     whether each endpoint is included.  Math expressions now carry interval
     operators such as `orderedInterval`, `unorderedInterval`, and
     `centeredInterval`, with `endpointInclusion` on the math expression
     itself.

     ```json
     {
       "math:1124": {
         "type": "mathExpression",
         "operator": "orderedInterval",
         "endpointInclusion": { "left": "inclusive", "right": "exclusive" },
         "operands": ["math:1123", "math:1122"]
       }
     }
     ```

128. Preserved scalar negation on mekso operators and operands.
     CLL 18.21 distinguishes `na'e su'i` from ordinary addition and
     `na'e bo mu` from the number 5.  The previous lowering erased the scalar
     negation and emitted ordinary `add` or literal `5`.  The affected
     `mathExpression` now carries `scalarNegation`, preserving the source
     marker and the `otherThan`/`opposite` distinction.

     ```json
     {
       "math:1124": {
         "type": "mathExpression",
         "operator": "add",
         "scalarNegation": { "kind": "otherThan", "introducedBy": "na'e" },
         "operands": ["math:1123", "math:1122"]
       },
       "math:1121": {
         "type": "mathExpression",
         "literal": { "kind": "integer", "value": 5 },
         "scalarNegation": { "kind": "otherThan", "introducedBy": "na'e" }
       }
     }
     ```

129. Parsed simple PA numeric forms in mekso values.
     CLL 18.19 and 18.22 rely on multi-digit and signed PA forms such as
     `pare`, `bize`, `reno`, `ni'umu`, and `pimu`.  Leaving these as opaque
     `"number"` text made equations such as `li ci na'e su'i vo du li pare`
     harder to inspect and obscured `li bize = 87`.  Simple PA integers now
     become integer quantity or math literal values; simple PA decimals become
     decimal textual math literals or quantity text values pending a dedicated
     arbitrary-precision decimal type.

130. Added `ni'e` selbri operands in math expressions.
     CLL 18.18 uses `ni'e ni clani`, `ni'e ni ganra`, and related forms to
     turn amount selbri into mekso operands.  Emitting
     `{ "literal": { "kind": "expression", "value": "mekso" } }` lost the
     source selbri and the amount abstraction.  `ni'e` operands now emit
     `literal.kind = "selbriOperand"` and point `denotes` at the explicit
     abstraction when the selbri supplies one.

131. Applied SE conversion to mekso operator operands.
     CLL 18.21 says `li ci se vu'u vo du li pa` means 3 subtracted from 4
     equals 1.  Keeping the operands in surface order made it look like
     `3 - 4 = 1`.  The builder now applies SE conversion to the math-expression
     operand order, so `se vu'u` emits `operator = "subtract"` with the first
     two operands swapped.

132. Pruned unreachable builder-only objects from public graphs.
     The public JSON graph is not an implementation trace: every object in
     `objects` must be reachable from `root`.  This required making several
     semantic edges explicit rather than relying on leaked temporary objects:
     displayed-content objects anchored to an utterance are listed in that
     utterance's `asides`; self-identification vocatives use the identified
     referent as `content`; and primary eventualities used as modal relation
     arguments point back to their defining formula through `content`.

133. Preserved bare-`jai` raised participants when `fai` fills the moved place.
     In examples like `do jai se krinu ... fai le nu mi lebna ...`, replacing
     the base relation's moved place with `abstractionAbout(do)` would erase the
     ordinary `krinu(reason, justified-event)` routing, but dropping `do` loses
     the surface x1 entirely.  The graph now keeps the base predication intact
     and conjoins an asserted constructed relation `involves(fai-event, do)`.

## Not Adopted From v0

The review also clarified what not to copy.

1. Do not make the JSON model Lean-shaped.  Lean helper types such as
   `Prop`, `DRS`, and polymorphic `Question alpha` are renderer details.
2. Do not treat a discourse sequence as logical conjunction merely because a
   target language wants one top-level proposition.
3. Do not make anaphora resolution a separate semantic object by default.  The
   resolved referent should be used directly; resolution traces can be
   provenance.
4. Do not collapse attitudinals, evidentials, discursives, or `kau` into
   comments.  They are displayed semantic content or question/focus structure.
5. Do not desugar lujvo into asserted component relations.  Keep lexical
   metadata separate from discourse truth.
6. Do not use anonymous existentials as a fallback for numbers, lerfu, or
   abstraction variables.  Preserve the math expression, sign, or parameter.

---

## Amendments — jbotci CLL Review Pass (2026-06-23)

These amendments were adopted after a CLL-wide review of `jbotci gentufa`/`tersmu`
(chapters 9, 10, 11, 14 + pilots; baseline = CLL as amended by xorlo). Each entry
states the decision, the concrete JSON shape, whether it **implements** an already-
prescribed treatment or **extends** the model, and the tracking issue. Remaining
implementation gaps are listed separately in “Known Implementation Divergences”.

1. **`ni` amount-abstraction scale place (#2) — implement.** The `ni` abstraction’s
   x2 (the measurement scale, CLL 11.5) is the **x3 of the constructed `amountOf`
   link**, exactly as a NU abstraction’s extra surface places shift to x3 of the
   abstraction link (cf. `su'u … be`). Emit the overt `be`-scale referent there;
   elided ⇒ its own `zo'e`. No schema change. Implemented in `tersmu` v1.

2. **Stacked / interleaved aspect-recurrence operators (#3) — extend.** Introduce a
   single ordered `intervalModifiers` stack on `eventuality`: a heterogeneous
   tagged list reusing the existing `Recurrence`/`Aspect`/`distribution` payloads,
   in **surface order = outermost-first**, holding exactly the contour-nesting
   operators (ZAhO/TAhE/ROI). PU/ZI/ZEhA remain positional scalar attributes.
   This makes `di'i co'a`, `ro roi … su'o roi`, and `ca'o` between two `roi`
   reorder-sensitive instead of byte-identical. (Within-collection order is already
   builder-preserved but undocumented — document it.)
   Implemented in `tersmu` v1.

3. **ROI count carries a QuantityForm (#5) — implement.** Route ROI counts through
   the existing first-class `quantity` object (`form` ∈ exact/atLeast/atMost/…,
   `scale:"frequency"`), referenced by id, instead of a bare `QuantityValue`.
   Retire the bespoke `recurrence.value` string path (which currently smuggles a
   form-name like `"all"` into the value slot — the rejected anti-pattern).
   Implemented in `tersmu` v1.

4. **`ki` sticky tense/space (#6) — extend.** Add an additive `sticky: bool` to the
   shared anchor-relation / temporal-path-step object (covers temporal and spatial),
   set on the frame that establishes the sticky default and on a bare-`ki` reset;
   plus an optional `inherited: bool` to distinguish a copied-forward frame from a
   freshly-stated one. No separate discourse-level register object — sticky state
   stays resolved-in-place (per design 0.B, mirroring resolved-anaphora-as-id).
   Implemented in `tersmu` v1.

5. **whether-or-not asserted operand (#10) — implement (per design 0.L).** `U`
   (`u`/`ju`) asserts the first operand and marks the second `mode=inert`; `se`
   exchanges, so `se ju` marks the **first** inert. Realize the asymmetry on the
   existing predication `mode=inert` value (left-inert vs right-inert); keep children
   in surface order and the surface marker in `connector.source`. Do **not** add an
   `assertedChild` field — `inert` is the model’s existing carrier.
   Implemented in `tersmu` v1.

6. **SE operand-swap on logical connectives (#12) — implement.** `se CONN` →
   `C(q,p)` uniformly across loci: exchange the children for symmetric operators
   (`and`/`or`/`iff`); for `U`, move the `inert` side (ties to #5). Stop writing the
   bare connective word into `truthTable`; either emit a genuine composed 4-bit truth
   table at **all** loci or drop the field as redundant with `operator` +
   `connector.source`. `tersmu` v1 emits the composed four-bit table.

7. **Canonical form for negated logical connectives (#14, #15) — implement; one
   change.** The design 0.L gloss “`na .a` = `imp`” describes the **truth function**,
   not a directive to emit an `implies` node; the canonical FRM is `C(not p, q)`.
   **Delete the `na ja` → `implies` special case.** All operand-negated logical
   connectives, at **every** locus (statement, sumti, bridi-tail, forethought),
   lower to the base-vowel operator (`or`/`and`/`iff`) with the relevant operand
   wrapped in a `not` formula.  Surface position determines the operand:
   `na ja` and forethought head `ganai` negate the first operand, while `ja nai`
   and forethought separator `ginai` negate the second.  The surface (`na ja`,
   `ja nai`, `ganai … gi`, `ge … ginai`, …) is recorded in `connector.source`.
   This makes the truth-table variants structurally parallel and removes the
   cross-locus inconsistency.
   Implemented in `tersmu` v1.

8. **Connective run grouping is binary and surface-mirroring (#18) — extend /
   document.** Associative runs of the **same** operator may be rendered as nested
   binary formulas mirroring surface grouping; **never** flatten across **distinct**
   operators (a `gi'e … gi'a` run is `(A∧B)∨C`, not a 3-child `and`). Pin binary,
   surface-mirroring nesting in the spec and align the afterthought builder with the
   already-correct forethought shape. (Pairs with the connective-correctness bugs.)
   Implemented in `tersmu` v1.

9. **Non-logical statement connectives (#7) — extend.** `.i joi` / `.i ce'o` etc.
   attach a **truth-valueless** `nonlogicalConnection` descriptor to the `sequence`:
   `{ operator, connector:{ source, locus:"statement" } }`, reusing the existing
   non-logical composition vocabulary (`joi`→mass, `ce`→set, `ce'o`→sequence,
   `fa'u`→respectively). It stays **out of `content`/FRM** (0.L: non-logical
   connectives never enter FRM) and out of `connectionClaims`. Implemented in
   `tersmu` v1.

10. **Assertoric subordination of connected operands (#11) — extend.** On items of a
    `content`-bearing `sequence`, emit `force:"subordinated"` so a consumer never
    double-asserts the operands of `.ija`/`.ijo`/implications; the single assertion
    lives in `content`. Operand predication `mode` need not change (the `content`
    edge already encodes subordination). Add a §sequence sentence making the
    “single combined claim” guarantee explicit. Implemented in `tersmu` v1.

11. **Modal over a connected formula / group (#16) — NOT adopted; closed.** JSON
    amendment #71 already specifies that one modal over a connected bridi-tail or
    `tu'e` group **spreads/duplicates** onto each predication via shared filler ids
    (no new object). Adding `modalArguments` to `formula`/`sequence` would contradict
    #71 and put an asserted relation onto a truth-valueless SEQ. The real defects on
    that path (`tu'e` force; deictic-ground duplication) are separate builder bugs.

12. **Tanru link typing (#8) — extend (narrow); do NOT add a first-class selbri
    object.** The model keeps the 0.F / §tanru-lowering desugaring (asserted tertau +
    `ka`-reified seltau + **vague unresolved `R`**); a recursive `selbri`/`tanru`
    object was deliberately rejected (Not-Adopted #1/#5). Replace the untyped
    `R[tanru:blanu-zdani]` **string** with a typed `tanruLink` sidecar on the link
    predication: `head` → tertau predication id, `modifier` → the `ka`/composite id,
    plus a display-only `relationLabel`. Same graph shape, same (deliberately vague)
    truth conditions, but head/modifier roles become machine-readable (removes an
    untyped-blob strong-typing violation). Implemented in `tersmu` v1: the link
    predication uses `relation:"tanru"` and the constituent label lives only in
    `tanruLink.relationLabel`.

13. **`lo'e`/`le'e` body non-veridicality (#9) — implement (the narrow real bug).**
    The kind/archetype **ontology** and matrix **genericity** asks are **not adopted**
    — design 0.D deliberately keeps `lo'e`’s relation to `lo` open and uses the
    `typicalDescription` descriptor kind as the sole marker (no `Kind`/`Archetype`
    sort, no generic predication mode: that would fabricate determinate structure the
    language underspecifies). But the body must be **non-veridical**: emit
    `veridical:false` on the descriptor for `lo'e`/`le'e`, marking the body as
    characterizing rather than veridically restrictive. Implemented in `tersmu` v1.

14. **`fa'u` respectively-distribution (#4) — extend.** A respectively-pairing is a
    **zip across parallel sequences**, whose correspondence is its entire truth-
    conditional content; a flat `respectively` `members` list cannot link it to a
    quantifier’s witnesses (14.124) or a second parallel list in another place
    (14.133). Add a declarative `respectivelyDistribution` FRM node: a reference-by-id
    `body` template + N parallel `streams`, a distinctness/partition flag for the
    quantifier-witness case, and `parameter.role="respectiveSlot"` placeholders.
    Keeps one reading; does not pre-flatten. Implemented in `tersmu` v1.

15. **`vau`-distributed shared terms (#20) — extend.** Coreference is shared id (0.B,
    no binder node). Make id-reuse for grammatically shared tail/head terms
    **normative** so an id-**inequality** becomes a detectable defect; add an optional
    provenance tag via the existing `ArgumentValue.source.construct`
    (`"shared-tail-term"`/`"shared-head-term"`). No new semantic node (Not-Adopted #3).
    Implemented in `tersmu` v1.

16. **`fi'o se pilno` place numbering (#17) — model correct; doc only.** Per 0.H the
    `ModalArgument.relation` carries the **unconverted** root (`pilno`) and the tagged
    sumti lands in the SE-remapped base place (x2 = tool); output is already correct.
    Resolved by documenting that the SE-remapping rule applies to
    `ModalArgument.arguments` as well as `predication.arguments` (primer §3.5). Closed.

17. **`quotation.mode` (#19) — model correct; doc only.** `mode` is the two-valued
    **category** (`parsed` vs `opaque` — the structured/reachable vs sealed split,
    0.O), with the delimiter recoverable from a separate field. The implementation
    matches this spec; only the primer mis-described `mode` as the delimiter cmavo.
    Resolved by correcting the primer. Closed.

18. **`abstractionAbout` & operand reuse (#21) — model correct; doc only.** `tu'a X` =
    a raised unspecified abstraction (0.I); CLL 11.10 deliberately discards identity,
    so nothing truth-conditional is lost. Resolved by primer edits (add
    `abstractionAbout` and the other operand-bearing descriptor kinds to the kind
    table; fix the `operand` field description). `tu'a` and bare `jai` raised
    abstractions carry the **`eventuality`** sort: they stand for an unspecified
    event/state/process abstraction about the operand, not for a proposition. Closed.

19. **Interval-modifier `-nai`: scalar (TAhE/ROI) vs contradictory (PU/FAhA/ZAhO)
    (#62) — implement TAhE/ROI; ZAhO already correct; reconcile a CLL erratum.** CLL is
    internally inconsistent: **CLL 15.7** lists "TAhE, ROI, **or ZAhO**" `-nai` as
    *scalar*, but **CLL 10.18** treats PU/FAhA/**ZAhO** `-nai` as *contradictory* (Ex
    10.132 `ca'onai` = "not-during …") and gives the *scalar* ZAhO form as the explicit
    `na'e ca'o`; 10.18's closing sentence lists only **TAhE and ROI** as the
    `-nai`-is-scalar case. We follow **10.18** (the more specific, self-consistent
    treatment).
    - **PU/FAhA/ZAhO `-nai` → contradictory** — a bridi-level `FRM` `not` over the atom
      (0.K), *not* a recurrence/interval scalar; *scalar* negation of such a tense is the
      explicit `na'e`-prefixed form (`na'e ca'o` → `scalarNegation: otherThan` on the
      aspect). **Already implemented correctly** (`ca'onai`/`ca'o nai` emit a `not`
      formula over the atom; `na'e ca'o` emits the scalar aspect) — no change needed.
    - **TAhE/ROI `-nai` → scalar.** Add a scalar variant (e.g. `otherThan`) usable on the
      `recurrence`/`intervalModifiers` negation, keeping the base recurrence kind and the
      count intact (`paroinai` = "other than once", CLL 15.79). *Current impl:* mislabeled
      `contradictory` (#62) — the single `contradictory` kind is never correct for TAhE/ROI.
    - *Type note:* `Recurrence.negation` is typed `ModalNegation`/`ModalNegationKind`; if a
      scalar variant is added there, rename/generalize it or document it as a **shared
      negation-shape type**, since interval-modifier negation is not a "modal" negation.
    - *Scale note:* the scalar reading is **scale-relative** — ROI `-nai` = "other
      than N" on a frequency scale, TAhE `-nai` = "other than this recurrence/distribution
      class" on a distribution scale.  Predication scalar negation now uses
      `scalarNegation.scale` for explicit or opaque scale referents; recurrence
      negation keeps the shared `negation` shape and its scale is the recurrence
      quantity/distribution itself.

20. **Metalinguistic `na'i` target and assertion effect (#53, #54) — extend.**
    `na'i` is displayed content, not formula negation.  Add
    `DisplayedContent.targetFocus` so leading `na'i go'i` can target the whole
    bridi while post-selbri `go'i na'i` targets the selbri/formula surface, and
    add `assertionEffect:"metalinguisticallyVoided"` so consumers know the host
    is not asserted true or false.  The affected host predications are inert.
    Implemented in `tersmu` v1.

21. **Statement-connective indicators (#55) — implement.** UI and related
    indicators attached to `.i je`/`.i ja` target the statement-connection
    formula, not either operand alone.  No new object type is needed; the
    displayed-content target points at the combined formula and uses
    `targetFocus:"bridi"`.  Implemented in `tersmu` v1.

22. **Scalar-negation argument scope (#57) — extend.** Add optional
    `scalarNegation.argumentScope`, a list of numbered places syntactically
    inside the scalar-negated selbri unit.  This distinguishes CLL 15.53
    trailing `lemi birka` from 15.54 `be lemi birka` without changing the
    predication's ordinary argument fillers.  Implemented in `tersmu` v1.

23. **First-class scalar scales and NAhE+BO definiteness (#61) — extend.**
    Add `scale` as a referent sort, `scalarNegation.scale`, `Descriptor.scale`,
    and `Descriptor.definiteness`.  `ci'u` supplies a scale definition via the
    scale referent's `descriptor.operand`; omitted scales are opaque scale
    referents.  NAhE+BO descriptors distinguish `indefiniteAlternative`
    (`na'e bo`) from `uniqueExtreme` (`to'e bo`), with `neutralPoint` and
    `affirmedPoint` for `no'e bo` and `je'a bo`.  Implemented in `tersmu` v1.

24. **Bridi-level `ja'a` affirmation (#65, #66) — extend.** Add
    `Formula.operator:"affirmed"` for NA selma'o `ja'a`.  This is a
    formula-layer affirmation wrapper and is deliberately distinct from
    predication-level scalar `je'a` (`scalarNegation.kind:"affirmed"`).
    Implemented in `tersmu` v1.

25. **`le`-series desugar to the `skicu` characterizing clause; drop the
    `veridical:false` flag for `le` (#123; supersedes #68 and the `le` part of
    amendment 13) — implement design 0.D.** The **primary** `le`/`lo` distinction
    under xorlo is **specificity** (carried by `descriptor.kind`:
    `speakerDescription` vs `veridicalDescription`), **not** veridicality;
    `le`'s non-veridicality is a **secondary, projective** property, not a
    main-bridi truth-conditional difference (guskant: "`le`'s logical property is
    the same as `lo`'s"). Per guskant / design 0.D,
    `le broda` = `zo'e noi mi ke'a do skicu lo ka ce'u broda`: the referent is a
    plural **constant** characterized by an **incidental** `skicu(speaker,
    referent, audience, ⟨ka ce'u broda⟩)`, with `broda` predicated of the `ce'u`
    **inside the `ka` abstraction** — never of the referent. So:
    - emit, for each `le`/`lei`/`le'i`, an incidental `skicu` predication +
      a `ka` abstraction (one `ce'u`, body `broda(ce'u)`), instead of
      `descriptor.body = broda(referent)`;
    - non-veridicality becomes **structural** (the referent is only ever `skicu`
      x2 = "described-as"), so the `veridical:false` flag is unnecessary for the
      `le`-series and is dropped.
    - **`la`** is the analogous case with a `cmene` clause
      (`zo'e noi lu broda li'u cmene ke'a mi` — the quoted word names the
      referent; #95/#119): preserve the name string as a first-class quoted
      sign in `cmene` x1 and desugar the `cmene` clause likewise rather than
      flagging `veridical:false`. `lo'e`/`le'e` are intensional
      typical/stereotype descriptors and keep their own treatment (out of
      scope).
    *Current impl:* `le` emits `descriptor.body = broda(referent)` (+ a
    `veridical` flag), reading as the referent veridically being `broda`.

26. **Canonical mass/set structural desugaring (#97/#98) — extend and pin one
    encoding.** Mass and set gadri are direct structural desugarings, not
    alternative descriptor flavors with ambiguous bodies:
    - `loi/lei/lai broda` = `lo gunma be lo/le/la broda`;
    - `lo'i/le'i/la'i broda` = `lo selcmi be lo/le/la broda`.
    The aggregate referent is the `gunma`/`selcmi` output.  The inner constant
    carries the member characterization, including the `skicu`/`cmene` treatment
    for `le`/`la`.  Keep `descriptor.kind` and `word` as provenance on the
    aggregate, but do not predicate the inner selbri of the aggregate itself.

27. **Direct abstraction outputs; no semantic-empty `abstraction` wrapper (#84
    and abstraction follow-up) — extend.** Public JSON no longer uses
    a separate abstraction wrapper whose only payload is `{ kind, body }`.
    Instead, the abstractor's output referent carries the body/content, binders,
    and real extra places directly.  For `nu`/`za'i`/`pu'u`/`zu'o`/`mu'e`, the
    output is the embedded predication's eventuality referent; `nu` uses broad
    `sort:"eventuality"` in the CLL 11.2/11.3 sense, while the aktionsart
    abstractors refine the sort to `eventuality/state`,
    `eventuality/process`, `eventuality/activity`, or
    `eventuality/achievement`.  For `ka`, the output relation carries
    `parameters` and `arity`; for `ni`, `jei`, `li'i`, `si'o`, and `du'u`, the
    CLL 11.13 extra places are fields on the output (`scale`, `epistemology`,
    `experiencer`, `mind`, `expressedBy`).  `su'u` is
    `sort:"abstractNature"` and has no CLL 11.13 x2; do not fabricate one.
    Public JSON does not emit `abstractionKind` or eventuality `class`.
    Connected event abstractors may still duplicate inert body formulas so each
    branch can have its own event-type view.

28. **Re-quantification of an established variable (#72/#73).** `re da` after
    `ci da` is an ordinary quantifier over a fresh selected variable whose domain
    is membership in the established witness set.  Quantified formulas use
    optional `sourceVariable`/`selectionSource`, keep the overt `quantity`, and
    preserve the inherited restriction from the original binding.  The body
    remains under the existing quantifier machinery, so scope, negation, and
    connective behavior do not get a second implementation path.

29. **Equal-scope grouping termsets (#75/#76).** A `ce'e`/`nu'i...nu'u`
    grouping termset cannot be lowered as ordinary nested quantifiers, because
    CLL 16.7 says the grouped terms have equal scope.  Use
    `operator:"quantifierBundle"` with ordered `bindings`, one shared body, and
    explicit `coequalScope:true` so consumers know the bindings have no defined
    relative scope.

30. **Structured `co'e` unspecified relation (#83) — extend.** `co'e` is the
    relation-position analogue of `zo'e`; do not emit a lexical relation string
    `"co'e"`.  Use a relation parameter or relation-placeholder object with
    `role:"unspecifiedRelation"` and `introducedBy:"co'e"`, then point the
    predication at that object through `relationParameter` or an equivalent typed
    field.

31. **Place-specific `ko` imperative target (#91) — extend.** `ko` still resolves
    to the addressee referent and the host utterance has command force, but each
    `ko` argument occurrence must also mark the concrete command target place.
    This distinguishes `ko klama` from `mi viska ko` without mutating the global
    addressee referent.  Use `ArgumentValue.commandTarget:
    {"introducedBy":"ko"}` on the filled argument occurrence.

32. **Outer quantifier representation (#99) — doc correction, later refined by
    amendment 41.**
    Delete the abandoned `ArgumentValue.quantity` account for outer quantifiers.
    Outer quantifiers are formula-level restricted-variable scopes; descriptor
    quantities remain for inner description quantifiers. The earlier claim
    that the import-free shape was fully "consistent with design C-9/C-22" was
    false for restricted universals: the scope structure was correct, but CLL
    16.8's projective domain import was still missing. Amendment 41 supplies
    that required marker without changing the formula-level scope decision.

33. **Quoted utterance use-status vs force (#100) — doc correction.** A parsed
    quotation preserves the quoted utterance's intrinsic force (`assert`, `ask`,
    `vocative`, etc.).  The fact that it is mentioned rather than performed is
    represented by the quotation/sign edge that contains it, not by rewriting its
    force to `mention`.

34. **Tanru connector locus spelling (#90) — doc correction.** The canonical
    connector locus for tanru formula links is `"selbri"`.  Stale examples using
    `"predicate"` are non-normative and should be updated or treated as a
    deprecated alias only if consumer compatibility requires it.

35. **MAI/MO'O ordinal labels (#114) — extend.** Numerical free modifiers headed
    by `mai`/`mo'o` are truth-conditionally inert labels.  Add an ordinal label
    on the affected sequence item: `mai` labels an item, `mo'o` labels a larger
    division.  Preserve the PA/lerfu value as a quantity or ordinal expression.

36. **`mo'e` quantified sumti operands (#115) — extend.** `mo'e <sumti>` must
    reference the full wrapped sumti denotation, including outer quantifier
    scope and cardinality.  If the wrapped sumti is formula-scoped, the math
    operand points at a quantified operand/scope object rather than only at the
    bare referent.

37. **`xi` subscripts (#117) — extend.** Add a reusable `subscript` value on
    symbols, referents/parameters, and math expressions where XI can attach.
    Subscripts are identity-relevant for variables and pro-sumti handles:
    `ko'a xi re` is a different assignable handle from bare `ko'a`.  Nested `xi`
    is sub-subscript; `ce'o` inside a parenthesized subscript is a same-level
    compound subscript sequence.

38. **`na'u` selbri operators (#118) — extend.** A `na'u` operator is a typed
    math operator backed by the underlying selbri relation and place structure.
    Preserve the convention from CLL 18.18: x1 is the result and later unfilled
    places are operands.  Do not stringify the selbri into an opaque operator
    label.

39. **Mixed-radix `pi'e` numerals (#121) — extend.** `pi'e` is number
    punctuation inside a positional/mixed-radix literal, not an arithmetic
    connective.  Add a structured literal with ordered components and optional
    `ju'u` base/radix, while preserving the surface source.

40. **Shared lowering for selbri-wrapper constructs (#126; parser bug #125)
    — extend.** Grammar productions that consume a full selbri outside the
    ordinary main bridi-tail must share the ordinary selbri lowering path:
    `(LA|LE) ... selbri`, `quantifier selbri`, bare-selbri vocatives,
    `FIhO selbri FEhU`, `NAhU selbri TEhU`, `NIhE selbri TEhU`, and
    `SEI [terms [CU]] selbri SEhU`.  These constructs preserve tanru, `be`
    linkargs, conversion, JAI, NAhE, connectives, elided/deleted places, and
    dictionary place structure in a subordinate predication/relation body.  No
    public raw `selbriExpression` object is added.  `fi'o` always uses a modal
    `body` formula, even for one-brivla tags such as `fi'o kanla`; `la <selbri>`
    preserves the name sign while allowing that sign to denote the lowered
    selbri relation body; `na'u` and `ni'e` point their math outputs at the
    typed lowered selbri output rather than opaque strings.

41. **Projective domain import for restricted universals (#279) — implement.**
    CLL 16.8 makes the restriction domain of `ro da poi ...` nonempty, but that
    commitment cannot be encoded as the ordinary at-issue conjunct previously
    prescribed by design 0.E. By CLL 16.9/16.11, moving `naku` across the
    universal yields a restricted existential and still entails a witness;
    negating `and(ALL(R -> B), EX(R))` instead introduces a catless-world
    disjunct `not(EX(R))`. The domain commitment must therefore project.
    Restricted `forall`/`pluralForall` formula nodes emit
    `domainImport:"projective"`; the field is absent from every other node.
    Restricted `exists`/`pluralExists`/`cardinality` retain their classical
    restriction-as-conjunct import, and `none` remains non-importing. This is
    the classical-divergence criterion: annotate only intended semantics that
    a naive classical graph reading cannot derive and structure cannot encode.

42. **Typed scope dependence for constants (#352) — extend.** Every constant
    referent emits `scopeDependence`. Binder-free introduction sites use the
    explicit shape `{"kind":"fixed"}`; sites under one or more typed binders use
    `{"kind":"underspecified","mayDependOn":[...]}` with a nonempty, sorted set
    of exactly those binder ids. `mayDependOn` states possible Skolem
    co-variation only; it does not assert actual dependence. Derivation follows
    typed formula, abstraction, question, and nested-utterance scope edges from
    the graph root, and the graph invariant recomputes it. Claims render this
    state on each constant denotation line rather than emitting an unqualified
    existential line. The lines remain projected annotations, not a fourth
    commitment tier, because dependence and commitment status are orthogonal.

## Known Implementation Divergences (2026-06-23)

Where current `tersmu` output departs from this spec (amended above). These are the
*model/encoding* divergences a future review should treat as **known**, not re-flag;
each is tracked. (Pure builder-correctness crashes/bugs — e.g. the connective
`na`/`nai` drops, termset Cartesian product, the `vo'a`-in-description stack overflow
— are tracked as `bug` issues, not here.)

- `fa'u` correspondence is unrepresentable (#4, amendment 14).
- Selbri-wrapper constructs are still lowered piecemeal; in particular,
  the shared `le`/`la`/vocative/`na'u`/`ni'e` lowering path is not implemented
  yet (#126, #123).

**Why the primer is a separate document.** `review/tersmu_schema_primer.md` is a
consumer/agent-facing cheat-sheet of the *current* JSON shape (used by the review
harness to interpret output). It is intentionally kept apart from this design spec so
it can track *what tersmu emits today* — including the divergences above — without
implying those shapes are endorsed. It carries a “do-not-flag” list pointing back here.
