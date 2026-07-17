# A Discourse Object Model for Lojban Semantics

### A flat, typed-graph representation capturing the *entire* semantic load of an arbitrary Lojban utterance

*Synthesized reference. Companion documents in this repository: the JSON format
specification [`semantic-model-spec.md`](semantic-model-spec.md) (which carries the
appended “Amendments — jbotci CLL Review Pass” and “Known Implementation Divergences”
sections) and worked encodings [`semantic-model-cll-encodings.md`](semantic-model-cll-encodings.md)
(CLL chapters 5–19 against this model).*

---

## Scope and ground rules

This document defines an **object model** — in the programming sense: a set of typed objects (nodes) with attributes and references — whose instances represent the complete meaning of a parsed Lojban text. The design goal is **ruthless fidelity**: every claim a Lojban utterance makes, *including the claims it makes by omission*, must appear explicitly in the encoding. The canonical example is `lo botpi` ("a bottle"): `botpi` is "x1 is a bottle/jar/flask for contents x2, of material x3, with lid x4", so `lo botpi` **asserts a lid exists** (an unspecified x4) — you would have to delete that place with `zi'o` to speak of a bottle without one. The same discipline applies to elided sumti, elided tense, the eventuality of every predication, the utterance act itself, and its speaker and audience: none of these is ever silently dropped.

**Authority.** CLL (*The Complete Lojban Language*) is authoritative for every construct **except the gadri**, where the **xorlo** reform applies, in the specific form developed in **guskant's** gadri commentary (`lo broda = zo'e noi broda`, gadri as plural constants — see Part 0.C). Where CLL is pre-xorlo (its chapter 6 implicit-quantifier table), it is not used for the gadri.

**Input.** The model takes a *parsed* text as input. The interior of the "magic words" (`ZO`, `ZOI`, `ZEI`, `BU`, `SI`, `SA`, `SU`, `FAhO`) is treated as **already resolved by a left-to-right token-rewriting prelude** (their modern definition; see Part 0.O). Likewise **anaphora is already resolved**: a pronoun is represented as a *shared referent id*, not as an unresolved pointer, so the lexical scopes that resolved it carry no residual meaning in the model.

**One reading, never disambiguation.** Where a construct is genuinely ambiguous between determinate readings, the model represents **one** reading; it does not enumerate scopings. Where a construct is genuinely *underspecified* by the language (topic-comment place, donkey anaphora, the `to…toi` operand), the model **exposes the gap** rather than fabricating a determinate filling — this is the throughline of the whole design.

**Classical-divergence criterion.** Add an in-band semantic annotation exactly
when the intended reading diverges from the naive classical reading of the
graph **and** no faithful structural encoding exists. Leave classically
derivable facts unannotated, and prefer a correct structural encoding whenever
one exists. Thus composed connective truth tables and inert `U` operands need
annotations, the old `veridical:false` flag on structurally desugared `le` does
not, and restricted-universal domain import needs the projective marker defined
in 0.E. This criterion prevents both silent non-classical commitments and
redundant metadata.

**Projection invariant.** Projection derives from a typed semantic trigger plus
its formula scope, never from `category = constant`. Generated predication
events and referential denotations therefore use different typed identities:
the former receive existential force from a formula/sequence owner edge, while
only the latter participate in referent-denotation projection.

---

# Part 0 — The object model

## 0.A Object kinds

Every object occupies **one line**, in the uniform syntax:

```
KIND id : attr=val, attr=val, …
```

| KIND | what it is | principal attributes |
|---|---|---|
| `UTT` | an utterance / locution | `force` (assert/ask/command/mention/quote/parenthetical/subordinated/vocative), `content`, `eventuality`, `speaker`, `audience`, `deicticGround`, optional `asides` |
| `EV` | an eventuality referent | `denotation`=generated-bound\|referential, slash-separated eventuality `sort`; typed time/space/aspect/recurrence fields; direct event abstractions carry `content` |
| `REF` | a non-event referent | `category`=constant\|variable\|indexical\|composite, `sort`, optional `descriptor`/`composition`; constants carry `scopeDependence`; direct non-event abstractions carry `body`/`parameters` |
| `PAR` | a free variable / gap | `sort`, typed `role`, `introducedBy` |
| `PRD` | a predication (atomic proposition) | `relation` or `relationParameter`, optional `eventuality`, numbered `arguments`, `modalArguments`, typed `mode` |
| `FRM` | a logical formula | atom, connective, quantifier, `quantifierBundle`, or `respectivelyDistribution`; optionally owns generated events through `boundEventualities` |
| `SGN` | a sign referent | public `type:"referent"`, `sort:"sign"`; `kind`, `text`/`letterals`/`quotation`, optional `denotes` |
| `SEQ` | a discourse sequence | ordered `items`, optional formula/question `content`, `connectionClaims`, typed paragraph `relation`, optional `nonlogicalConnection`/`elidedConnectionOperand`; may own generated events when no formula LCA exists |
| `DSP` | a displayed object (attitudinal/evidential/metalinguistic/emphasis) | `family`, `relation`, `experiencer`=⟨REF-id⟩, `target`=⟨id⟩, optional `targetFocus`=bridi\|selbri, `anchor`=⟨UTT-id⟩, `intensity`, `polarity`, `phase`, `modifiers`, `assertionEffect` |
| `MEX` | a mathematical expression (mekso operand language) | `operator`, `operands`; or a typed `literal`; questioned operators use `operatorParameter` |
| `QTY` | a quantity | `form`, exactly one value representation, `scale`; `questionParameters` on `xo` values |
| `REL` | relation-level metadata (for lujvo) | `relation`, `sourceWords`, `placeStructure`, optional typed `expansion` — **annotation only, never part of any `FRM`** |
| `QST` | a direct or indirect question | `kind`, `mode`, `domain`, `body`, ordered answer `slots`; heterogeneous questions use `kind=multiple` and typed slots |

The notation above is conceptual.  The public JSON shape in
[`semantic-model-spec.md`](semantic-model-spec.md) emits every denotable value
as `type:"referent"`: events and states are referents with sort paths such as
`eventuality`, `eventuality/process`, or `eventuality/locution`; signs are
`sort:"sign"` referents; `ka`/`du'u`/`ni`/`su'u` outputs are referents whose
sorts are `relation`, `proposition`, `amount`, `abstractNature`, etc.  The JSON
ID prefix is derived from that sort path, and its numeric suffix is globally
unique across the graph.

A note on what is *not* a kind: there is **no anaphora node**, **no identity-binder node**, **no public abstraction wrapper**, and **no scope node for referent identity**. Coreference is *shared id*. Logical quantifier `FRM` nodes bind variables over formulas, direct abstraction-output referents bind parameters over their `body`/`content`, and the generated-event owner edge below binds an event witness at an existing `FRM` or `SEQ`; none introduces a generic scope object. Identity/anaphora scope is not represented because it is not needed once anaphora is resolved.

## 0.B Notation conventions

**Reference by id (canonical).** A compound formula never *contains* its sub-formulas literally; it **references them by id**. So `EX(X, f7)` is a quantifier node whose body is whatever `FRM f7` is, written on its own line. This is what keeps arbitrarily deep structure — a nested `to…toi`, a quotation of a quotation, a stack of abstractions — **flat**: every predication, every formula, every reified body is its own referenceable line, and nothing is buried inside a giant literal term. The same id-reference style is used everywhere the model already nests (`UTT.content=f1`, abstraction-output `REF.body=fd`/`EV.content=fd`, `DSP.target=…`).

**`args` entries** are: an object id; or `lae[id]` / `lue[id]` (the `la'e`/`lu'e` reference↔referent shifts); or `(id ⊕ id)` (a `jo'u` plural-sum composite). An **omitted place is never silent**: it is its own `REF …: category=constant, descriptor={kind:elided, word:zo'e}` line (the `botpi`-lid principle).

**`mode` on a `PRD`:**
- **asserted** — at-issue; appears in the utterance's `content` FRM; sits under truth operators.
- **incidental** — a `noi`-style side-claim; true and import-bearing, but **projective** (escapes at-issue operators) and **not** in `content`.
- **displayed** — use-conditional (the UI/evidential tier, 0.J); projective; anchored to the utterance event `e0`; carries no truth value.
- **restrictive** — a `poi`-style clause that **co-determines which plurality the referent is** (extension-fixing); travels with the referent.
- **inert** — present and import-bearing but **truth-functionally vacuous** (the second operand of `U`, "whether or not"; 0.L).
- **performative** — made true by the act of uttering (the `ca'e` case; 0.J).

**Implicit-value surfacing.** A bare event line `EV e1` carries the standing default **`tense=?`** (a *free contextual temporal reference*, resolved by context — explicitly **not** "present"). Omitted CAhA asserts no actuality status: `ca'a`, `ka'e`, `nu'o`, and `pu'i` constrain the already-bound witness only when explicit. An elided inner sumti is a `zo'e` `REF`; an elided outer quantifier on a constant is *nothing* (xorlo, 0.C). I write the defaults out in full in the companion encodings.

### Generated-event identity and binding

Every eventuality has one of two disjoint typed identities:

- A **generated-bound** eventuality is introduced by predication lowering. It
  has no referential `category` or `scopeDependence`, and exactly one existing
  semantic owner lists it in `boundEventualities`. The owner is the lowest `FRM`
  dominating every primary-field, argument/modal, transitive-tanru,
  formula-event, and reified-content use. If no formula LCA exists, the lowest
  containing `SEQ` owns it. Shared events are consequently bound once, outside
  every use, rather than once per atom.
- A **referential** eventuality denotes a Lojban/discourse object and retains
  ordinary referent category and scope dependence. `lo`/`le nu` denotations
  (including an embedded event promoted as the abstraction result), locution
  events, event indexicals, and mentioned event fragments remain entirely
  outside generated-event binding.

The owner edge supplies existential scope, not actuality. In particular, the
event of `mi na klama` is bound on the inner atom under `not`; `mi ca'a klama`
has that same binding shape plus an explicit actuality constraint on the
witness. Co-variation of a generated witness with outer logical binders follows
from graph nesting, so duplicating that fact as `scopeDependence` would be both
redundant and liable to disagree with the owner edge.

**The utterance is always present.** Every freestanding example is wrapped in
```
UTT u1 : force=assert, content=f1, ev=e0, speaker=a1, audience=a2, deictic-ground={time:n0, place:h0}
REF a1 : category=indexical, indexical=speaker, sort=entity
REF a2 : category=indexical, indexical=audience, sort=entity
EV  n0 : denotation=referential, category=indexical, indexical=now, sort=eventuality
REF h0 : category=indexical, indexical=here, sort=entity
EV  e0 : denotation=referential, sort=eventuality/locution, actuality=actual
```
In JSON, the top-level frame is emitted as ordinary globally numbered
indexical referents: `entity:1` speaker, `entity:2` audience,
`eventuality:3` now, and `entity:4` here.  Sibling top-level `.i` utterances
share that full frame; parsed quotations allocate a fresh speaker, audience,
now, and here for the quoted utterance.  Speaker and audience exist *because
the utterance does* — they are role-arguments of an asserted locution and so
carry existential import by 0.K/C-13 (Part 0.P).

## 0.C Referents are plural constants (guskant basis)

A `const` denotes a **plurality** (one or more) with **no inherent quantifier, no inherent distributivity, no inherent set-hood**. Two laws govern it:

- **Existential import.** `C broda` ⟹ `su'oi da broda` — a constant always has a referent. Correspondingly "there is none" (`naku su'oi da`) is *not* expressible by any `lo`-term, and **`lo no broda` is meaningless**.
- **Skolem reading.** A constant standing in the scope of a bound variable may **co-vary** with it (it is a Skolem function of the variable). This is exactly why every elided `zo'e` is surfaced as its *own* object and need not denote the same thing across a quantifier's range (`ro mlatu cu jbena zo'e zo'e zo'e` — every cat is born to its own parents, at its own time and place). The canonical graph records this on every constant as typed `scopeDependence`: explicit `fixed` when its introduction site has no binders, otherwise `underspecified` with the exact nonempty `mayDependOn` binder-id set. The latter records only possible dependence; it does not assert that the denotation actually varies.

`scopeDependence` is derived mechanically from the rooted typed graph, never from
source text or labels. Quantifier variables bind their restrictions and bodies;
coequal bundle/respective variables bind their restrictions and shared body;
abstraction parameters bind their body; question slots bind their question body;
and nested utterances reset the binder environment. The first reference to a
constant in canonical semantic traversal order is its introduction site; later
shared-id references keep that value. Normal generated graphs are connected after
pruning; any disconnected component in a hand-built graph is treated as an
ID-ordered root at empty scope. The graph invariant recomputes this derivation.

Primitives over constants:
- **`me`** — the among-relation. `me(e; y, X)` = "y is among X". Reflexive and transitive.
- **`jo'u`** — plural sum. `(X ⊕ Y)` is a new constant; commutative, idempotent, with `X me (X⊕Y)`.
- **`du`** — identity, definable as **mutual `me`**. The one place coreference is *claimed in the content* (rather than encoded as shared id): `du(e; a, b)` says "a is b" without having merged the nodes.
- **individual** ≡ `pa mei`: `X` is an individual iff `ro'oi da poi me X zo'u X me da` (equivalently `X pa mei`). `mi`, `ti`, `lo pa broda` are individuals; a substance/material referent (bread, where a cut piece is still bread) need not be.

## 0.D The gadri, expanded exactly

Each lexical occurrence introduces a fresh `category=constant` referent plus a
typed descriptor. Veridical descriptor restrictions and the property relation
inside a speaker description use `mode=restrictive`; the speaker
description's outer `skicu` predication is incidental. `lo`, `le`, and `la`
are plural constants, but their descriptor kinds and bodies differ.

| surface | objects introduced |
|---|---|
| `lo broda` | `REF x: category=constant, descriptor={kind:veridicalDescription, word:lo, body:f}`; `f` contains `broda(x)` in restrictive mode |
| `le broda` | `REF x: category=constant, descriptor={kind:speakerDescription, word:le, body:f}`; `f` contains incidental `skicu(speaker, x, audience, ⟨ka ce'u broda⟩)`, whose direct relation-sort x4 has a restrictive `broda(ce'u)` body — abbreviated **`LE-clause(x, broda)`** |
| `la cmevla` | `REF x: category=constant, descriptor={kind:name, word:la, name:cmevla}`; name-word lowering retains the corresponding `cmene` structure when applicable |
| `lo PA broda` | as `lo`, with `descriptor.quantity` pointing at the first-class PA quantity |
| `loi broda` | `REF m: category=constant, sort=mass, descriptor.kind=veridicalMassDescription`; its restrictive body relates the mass to the inner `lo broda` referent |
| `lo'i broda` | `REF s: category=constant, sort=set, descriptor.kind=veridicalSetDescription`; its restrictive body relates the set to the inner referent |
| `lo'e broda` | `REF x: category=constant, descriptor={kind:typicalDescription, word:lo'e, veridical:false}` — an intensional generic; its formal relation to `lo` remains exposed |

where `⟨ka ce'u broda⟩` is a direct `sort=relation` referent whose `body`
contains `broda(ce'u, …)` and whose `parameters` list that slot. The
non-veridicality of `le` is structural: the descriptor body says `skicu`, not
`broda(x)`. Public JSON does not serialize the internal abstraction-kind
discriminator; the output sort and its `body`/`content` fields carry it.
Consequently, `lo mlatu cu gerku` combines a veridical `mlatu` restriction
with the asserted `gerku` predication, while `le mlatu cu gerku` can describe a
dog as a cat without asserting that the description is true.

Place structures used by the expansions (community dictionary, verified): `gunma` (x1 is a mass/aggregate of components x2, jointly); `selcmi` (x1 is the set whose members are exactly x2); `skicu` (x1 describes x2 to x3 as x4); `cmene` (x1 = quoted word(s) is the name of x2 used by namer x3).

## 0.E Quantification (canonical reference-by-id form)

Logical quantification is **truth-conditional** (`∀∃ ≠ ∃∀`, and negation boundaries depend on it), so it *is* represented — as **operator nodes in the `FRM` graph that reference a body formula by id**:

```
FRM := ⟨PRD⟩                              -- atom
     | not(⟨FRM⟩) | and(⟨FRM⟩,…) | or(…) | imp(f,f) | iff(f,f) | …   -- connectives (0.L)
     | EX(v, ⟨FRM⟩)     -- ∃ : da, su'o
     | ALL(v, ⟨FRM⟩)    -- ∀ : ro
     | CARD(v, n, ⟨FRM⟩) -- exactly/at-least n : PA
     | EXP(v, ⟨FRM⟩)    -- plural ∃ : su'oi
     | ALLP(v, ⟨FRM⟩)   -- plural ∀ : ro'oi
     | NO(v, ⟨FRM⟩)     -- no = not∘EX
```

The bound `v` is still a global `REF kind=var`; the quantifier node binds it over the *referenced* body. **The body is a formula id, never a literal subtree** — this is what avoids lexically nesting deep `to`/quotation content inside a quantifier.

- **What global uniqueness gives for free:** capture-freedom (no `da` is ever reused, so every mention is unambiguously the same variable) and conditional existential import (a `PRD` mentioning `v` contributes import only where it is asserted-and-undefeated — 0.K). These need **no** explicit annotation.
- **What must still be stated explicitly:** the **binding edge itself** — `EX(v, ⟨f⟩)` — because its *type* (∃/∀/n), its *scope* (which formula), and its *position relative to other binders and to `not`* are precisely the truth-conditional facts that uniqueness does **not** supply. Dropping the binding edge would make `∀∃`/`∃∀` and "no dog comes"/"some dog doesn't come" unrepresentable, and would leave nowhere to record whether a restriction is an antecedent (∀) or a conjunct (∃).
- **A prenex** `[v1 v2 …] zo'u body` is the nesting in **surface order**: `Q1(v1, ⟨g1⟩)` with `g1` = `Q2(v2, ⟨g2⟩)`, etc. Surface/left-to-right is the single reading (the CLL default when a prenex is dropped).
- **Restricted quantification** (`da poi broda`): ∃, plural ∃, and cardinality
  restrict via ordinary at-issue conjunction — structurally,
  `EX(v, restriction=broda(v), body)` means
  `EX(v, ⟨and(broda(v), body)⟩)`. Restricted ∀ uses the distinguished
  `restriction` as the implication antecedent, but also carries
  **projective domain import**: the quantifier node has
  `domainImport=projective`, meaning `EX(v, broda(v))` survives operators such
  as `not`. It is not an extra at-issue conjunct. CLL 16.8 requires this
  nonempty domain for `ro da poi klama`.

  The previously prescribed at-issue encoding
  `and(ALL(v, imp(broda(v), body)), EX(v, broda(v)))` is wrong by `naku`
  duality. Let `R=mlatu` and `B=klama`. CLL 16.9/16.11 equate
  `naku ro da poi R cu B` with the quantifier-inverted
  `su'o da poi R naku cu B`, whose classical content is
  `EX(v, and(R(v), not(B(v))))` and therefore still entails that an `R`
  exists. Negating the conjunct encoding instead yields
  `not(and(ALL(v, imp(R(v), B(v))), EX(v, R(v))))`, equivalent to
  `or(EX(v, and(R(v), not(B(v)))), not(EX(v, R(v))))`; it is true when no
  `R` exists and is therefore strictly weaker. Projecting the domain
  commitment while negating only the at-issue universal gives exactly the
  `naku`-moved reading.

  This also distinguishes the genuinely import-free **"any"** of CLL 16.8:
  `ro da zo'u da go broda gi brode` has `broda(v)` inside the body's
  biconditional and no quantifier `restriction`, so it carries no marker and no
  restricted-domain import.
- **Re-quantifying an established da-series variable** (`ci da ... pa da`) introduces a fresh selected variable, not a second quantity on the same bound variable. The selected variable's quantifier records the source variable / witness set, and any source-domain restrictions such as `poi broda` are copied as restrictions on the selected variable. This keeps "one of the three people" distinct from the set of three while still exposing where the selection came from.
- The **existential "any"** (CLL 16.50, "I need any box bigger than this") is a variable bound in the prenex of a **subordinate** bridi: `nitcu(a1, ⟨nu EX(X, and(tanxe(X), bramau(X, ti), ponse(a1, X)))⟩)` — the `EX` scopes inside the `nu` body (its own formula id), so the box's existence rides only on that (possibly non-occurring) event, not on reality. A variable defaults to the prenex of the **smallest enclosing bridi**; an outer prenex must be explicit to widen its scope (CLL 16.8/16.11).
- **Inner quantifier** (`lo PA broda`) is **not** logical quantification: it is the `mei`-count of one constant's individuals (0.D).
- **Outer quantifier** `PA ⟨sumti⟩` = `PA da poi me ⟨sumti⟩`: a restricted bound **singular** variable, **distributive** by default (so `ci lo prenu cu jmaji`, "three gather", is anomalous). `su'oi`/`ro'oi` give bound **plural** variables (`EXP`/`ALLP`). Encode the restriction as a `PRD me(v, x) mode=restrictive`.
- **Grouping termsets** (`ce'e`, bare `nu'i...nu'u`) make their quantified terms coequal in scope (CLL 16.7). They are not either nested order. Encode them as one coequal quantifier bundle with ordered bindings and one shared body.
- Negation interacts by graph position: `not(ALL(v,⟨f⟩)) ≡ EX(v, ⟨not(f)⟩)` and `not(EX(v,⟨f⟩)) ≡ ALL(v, ⟨not(f)⟩)`. A distinguished restriction travels with the inverted quantifier. A restricted universal's projective domain import remains outside `not`; after inversion the resulting restricted existential entails the same nonemptiness classically and needs no marker. These are equivalences; the model records the **surface** form, and the two encodings are each faithful (0.K).

## 0.F Tanru desugar; lujvo do not

**Tanru** desugar in the `FRM` under a **single uniform schema**. The tertau `T` carries the primary meaning, dictates the place structure, and supplies the **one** eventuality of the bridi; the seltau is a **modifier**, not a free-standing predication:

> `T(eT; x, …) ∧ R[tanru](x, ⟨ka ce'u S⟩)`

The seltau is reified as the kind/property `⟨ka ce'u S⟩`, a direct
`sort=relation` referent whose non-`ce'u` places are existentially closed
inside its body. A typed `tanruLink` connects `x` to that property. This
asserts neither `S(x)` nor a concrete seltau referent:
the seltau-tertau relation is genuine underspecification, retained structurally
rather than fabricated. Only the tertau event is the tanru head event; events
inside the modifier relation remain scoped in its body. Connected tanru units,
`be`/`bei`, `co`, and `SE` retain their structural place and event behavior.

In JSON, the vague link predication uses the stable relation label `tanru` and
a typed `tanruLink` sidecar (`head`, `modifier`, `relationLabel`) rather than
encoding the constituent structure in the relation string.
For `barda nanla`, the asserted `nanla` predication is the head and the direct
relation-sort `barda` property is the modifier; the `tanru` predication and its
`tanruLink` preserve that asymmetry. `sutra bajra` has the same shape with
`bajra` as head and the `sutra` property as modifier. Even a conventionally
intersective example such as `remna nakni` retains this uniform shape; the
model does not guess a special tanru reading from lexical familiarity. The
same representation handles asymmetrical `cinfo kerfa` without asserting that
the mane is a lion, and likewise leaves the contextual link in `rokci cinfo`
or `junla dadysli` explicit rather than inventing a concrete modifier
referent.

**Lujvo** do **not** desugar in the `FRM`. A lujvo's `rel` is an atomic
relation symbol with its own place structure; `REL` metadata records its
mechanical veljvo decomposition, place identifications, dropped/retained
places, and any implicit direct abstraction output. This metadata explains the
relation but never adds asserted formula children.

## 0.G Tenses are eventuality attributes

A tense/space cmavo is **never a numbered place**; it is an **attribute on the `EV`**:
- `PU` (pu/ca/ba) → `time`/`timePath` anchor relations; `ZI`
  (zi/za/zu) is the typed distance on the relevant path step.
- `ZEhA` (ze'i/ze'a/ze'u) → `timeInterval`; `VEhA`/`VIhA` and
  FAhA/VA populate `spaceInterval`, `space`, and `spacePath`.
- `TAhE` and `ROI` become typed `Recurrence` values, with first-class
  quantity objects and ordered `intervalModifiers` when surface ordering must
  be preserved.
- `ZAhO` becomes `aspect`/`aspects` and ordered interval modifiers; `CAhA`
  becomes `actuality.kind` (`actual`, `capable`, `potential`, `demonstrated`).
- `KI` → a **sticky default**: the value is copied onto subsequent `EV`s until reset. `cu'e` → a tense **question**: a `PAR role=tenseQuestion, sort=tenseModal` over the `EV`'s tense field. Tense **as sumtcita** (`ca lo nu broda`) → a predication relating `e1`'s time to another event's time (a `cabna`-style link, i.e. a modal anchored to another event).

## 0.H Modals are typed arguments of the host predication

A **modal tag** (`fi'o`/BAI) becomes a `ModalArgument` in the host
predication's `modalArguments` array. Its `relation`, `introducedBy`, and
canonical root-relation `arguments` retain the modal gismu and place mapping:
- ρ is **internal** to the tag gismu when it has an event place — `bai`←`bapli` puts the main event `e1` directly in `bapli`'s event place; likewise `ti'u`←`tcika`, `sepi'o`←`pilno` (x3).
- ρ is **external** otherwise — `fi'o kanla le zunle` retains the `kanla` arguments plus a contextual link to `e1`; `do'e` keeps the bare vague modal relation.

`FA` (fe/fi/fo/fu) = arg-slot reassignment, recorded resolved in `arguments`.
`JAI` = a relation conversion **raising an abstraction-place into x1** while
the graph retains the unconverted root relation and keys (`jai gau` raises the
agent specifically). `KI` = sticky modal (copied onto later predications).
Modal **sentence** connection — the causals `ri'a`←`rinka`, `mu'i`←`mukti`,
`ni'i`←`nibli`, `ki'u`←`krinu` — attaches a causal predication formula through
the sequence's `connectionClaims`; `nonlogicalConnection` retains the surface
connector. For afterthought `A .i ri'a bo B`, CLL 9.7 makes B the root x1 cause
and A the root x2 effect.

Because an ordinary modal is part of the host predication, contradictory
negation scopes over the host together with its modal claim. The graph does
not fabricate a second free-standing formula child solely to encode the tag:
`mi na klama bai do` negates the going-with-compulsion claim as a unit.

## 0.I Direct abstraction outputs

NU-family forms produce direct referents with a referenced `body` (non-event
outputs) or `content` (eventuality outputs), and any
`parameters`/`embeddedQuestions`. The sort identifies the abstraction kind in
public JSON:
- `nu` and the Aktionsart quartet `mu'e` (point), `pu'u` (process, +stages), `zu'o` (activity, +repeated actions), `za'i` (state) produce the **event referent itself**; its `content` points at the abstracted formula. There is no public `abstracted` wrapper field.
- `ka` abstracts **`ce'u` `PAR`s**; **multiple `ce'u` are distinct** (a relation abstraction), unlike `ke'a` (which is a reused, identical head referent). The **arity** of the resulting relation is the number of distinct `ce'u`: one `ce'u` gives a 1-place property, two a 2-place relation, and so on (so `le ka ce'u prami ce'u` is the 2-place *loving* relation, **not** "loves itself").
- `du'u` abstracts **nothing** (a proposition); factivity belongs to the matrix verb (`djuno`), not to `du'u`. Its x2 — **`se du'u`** — is **the sentence (a `SGN`) expressing the proposition**, the sign/sense bridge.
- A referent **over** an abstraction takes the **sort** of that abstraction's output: `nu`/Aktionsart → `Ev`; `ka` → `Rel` (a `Property`, arity as above); `du'u` → `Proposition`; `jei` → `TruthValue`; `ni` → `Amount`; `si'o`/`li'i`/`su'u` = idea/experience/generic, the mind-relative ones carrying `mind=⟨REF⟩`.
- `kau` inside a `du'u` turns the questioned place into a `PAR role=argumentQuestion`; the proposition lists the resulting indirect `question` in `embeddedQuestions`.
- `tu'a X` = a raised `abstractNature` referent with a body involving X; the
  converse `jai` raises it back at the selbri level.

## 0.J The displayed tier (attitudinals & evidentials)

Every UI/CAI indicator is a **`DSP`** object — projective, use-conditional, **bearing no truth value, never in `content`/`FRM`, never under truth-operators**:
```
DSP id : family=⟨emotion|propositionalAttitude|evidential|metalinguistic|emphasis⟩,
         relation=⟨emotion-label (pure) | gismu (evidential) | marker⟩,
         experiencer=⟨REF⟩, target=⟨id⟩, anchor=⟨UTT id⟩,
         intensity=⟨cai|sai|ru'e|cu'i⟩, polarity=⟨+|cu'i|nai⟩, phase=⟨bu'o:start|continue|end⟩
```
- **Scope/target by placement:** sentence-initial or post-selbri indicators
  target the relevant formula and retain `targetFocus=bridi|selbri` when that
  distinction matters; an indicator immediately after a sumti targets that
  sumti.
- **Pure emotion** (`.ui`/`.oi`/`.iu`…): host bridi **stays asserted**; the `DSP` displays an emotion about the target.
- **Propositional attitude** (`.ai`/`.au`/`.e'a`/`.ei`/`.e'u`…): host bridi is **subordinated** — reified as the attitude's `target` (a potential world) and **absent from the asserted `FRM`** (`content` carries no at-issue conjunct from it). The proof: to actually assert the bridi you must split the indicator into its own sentence.
- **Evidential** (gismu-linked: `za'a`←`zgana`, `ti'e`←`tirna`, `pe'i`←`pensi`, `ja'o`←`jalge`, `ru'a`←`sruma`, `ba'a`←`balvi`, `su'a`←`sucta`, `ka'u`←`kulnu`, `se'o`←`senva`, `ju'a`←`jufra`; plus `ca'e` define): a `DSP` relating the **speaker to the proposition**, setting its evidential force — *holds-from-source* (bridi asserted, source-marked, "indisputable"), *relayed/assumed* (`ti'e`/`ru'a` — bridi **not** speaker-asserted, sits in the gismu's `du'u` target), or *performative* (`ca'e` — bridi **made true by the utterance**, `mode=performative`).
- **`dai`** shifts `experiencer` to a non-speaker (or empathic); **`pei`** makes the emotion-type/intensity a `PAR role=attitudeQuestion, sort=entity`; **`ge'e`** = explicit null; **`bu'o`** = `phase` (the displayed-tier analogue of `ZAhO`, distinct from an asserted `ba'o prami` claim). **Insincerity = infelicity, not falsity** (there is no truth value to negate).

## 0.K Negation has three kinds, in three strata

- **Contradictory** — `na` (pre-selbri) / `naku` = `FRM`-level **`not(⟨FRM⟩)`**, always whole-bridi, always contradictory, doubling cancels. It lives at the `FRM` node of whichever bridi it sits in, including direct abstraction bodies and descriptor clauses. Internal `naku` is a negation boundary: crossing a quantifier inverts it (`∀↔∃`), crossing a connective forces DeMorgan; the model records the surface reading.
- **Scalar** — `na'e`/`no'e`/`to'e`/`je'a` leave the predication's `relation` unchanged and add its typed **relation-level** `scalarNegation` sidecar (`otherThan`, `opposite`, `midpoint`, or `affirmed`). These are **positive assertions** (select a different scale-point), strictly distinct from contradictory `na`. The scale is a first-class `REF sort=Scale`: `ci'u` can supply its definition, while an omitted scale remains opaque.
- **Metalinguistic** — `na'i` (a UI discursive) = a **`DSP family=metalinguistic`** marker, **not** a `FRM` `not`. It flags the bridi/term as mis-posed / resting on a false presupposition; no truth value.

**Existential import (formula-positional).** Import attaches to a **predication-occurrence** (an implicit `su'oi da poi me x` introduced at the predication's node) and propagates through `FRM` exactly as that predication's truth value does. A bare `REF` is **inert**. `zi'o` (a deleted place, ∅) and `PAR` carry no import; a bound `var` carries it only through its own quantifier, at its formula position. `incidental`/`displayed` predications **do** carry import but **project past** at-issue operators.

## 0.L Connectives are the `FRM` layer

All **logical** connectives, wherever they sit grammatically, desugar to the **same** `FRM` truth-functional structure; the grammatical locus only fixes **how much is shared/distributed**:
- **sentence** (`.ije`-series) — two complete `FRM`s joined; **bridi-tail / compound bridi** (`gi'e`) — an **explicit** x1 (and the prenex) is **shared** by both tails, but an **omitted** x1 is **not** shared: each tail supplies its own `zo'e`, so the goer and the walker of `klama … gi'e dzukla …` need not be identical (CLL 14.57–14.58); **sumti** (`.e`) — the connective **distributes over the shared predication** into branches that differ in the connected argument, keep overt non-connected arguments shared, and allocate independent `zo'e` referents for omitted non-connected places; **tanru** (`je`) — both predicates of the **same x** (the one case that need not reduce to a sentence connection); **forethought** (`ge…gi`, `gu'e…gi`) — same functions, prefix order; **termset** (`nu'i…nu'u` + `pe'e`) — **parallel** connection of several places at once, with non-connected surrounding terms replayed per branch so unequal-length termsets can shift a following term to a different numbered place (CLL 14.74).

Realization algebra: four vowels **A=`or`(TTTF), E=`and`(TFFF), O=`iff`(TFFT), U=`whether-or-not`(TTFF)**, plus **`na`** (negate first), **`nai`** (negate the operand selected by surface position), **`se`** (exchange) generate 14 of the 16 functions. Mapping: `naCONN`→`C(not p, q)`, afterthought `CONNnai`→`C(p, not q)`, forethought head `CONNnai ... gi`→`C(not p, q)`, forethought separator `... ginai`→`C(p, not q)`, `seCONN`→`C(q, p)`; e.g. `na.a` and `ganai ... gi` are emitted as `or(not p, q)` while their truth table is implication. **U** asserts the first and marks the second `mode=inert` (truth-vacuous but discourse-present, import-bearing).

**Connective questions** (`ji`/`je'i`/`gi'i`/`ge'i`) use a
`PAR role=connectiveQuestion, sort=connective` referenced by the connective
node and the question slot. **Non-logical connectives** (JOI/JOhI) do **not**
enter `FRM` — they build typed composite referents; statement-level forms use
`Sequence.nonlogicalConnection`. When a non-logical termset associates tagged
terms with different composite members, the modal relation records the relevant
member via `component`.

## 0.M Letterals are sign referents and anaphoric names

A lerfu word/string is a **sign-sort `REF kind=letteral`**. Its public
`letterals` array preserves each typed unit (`glyph`, `digit`, `shift`,
`characterCode`, or `compound`), including its source words, rendered text,
value where one is defined, and `buDepth` where applicable. Thus `gy.` carries
the glyph value `g`, while `denpa bu` remains a glyph unit sourced by the words
`denpa`, `bu`; the public graph does not synthesize a relation-destructuring
object merely because the source contains a brivla.

The same surface lerfu has three uses. **Character mention** (`me'o gy.`)
returns the sign referent. **Pro-sumti** use resolves an assigned name or
name-initial anaphor (`gy.` after `gerku`) to the already existing referent id;
there is no public `handle` field. **Mekso-variable** use is represented inside
the `MEX` literal as `{kind:"variable", value:"gy"}`. A lerfu string is one
sign referent or one variable name, not one referent per character.

## 0.N Mekso (the operand language)

Numbers are digit strings denoting values; the mekso operand language is **self-contained**, touching the core bridi layer through exactly four doors:
- **quantifier** (sumti-prefix) → `CARD`/`PA` in a quantifier FRM-node (0.E);
- **value term** `li [mekso]` → a number-sort `REF` whose number descriptor points to a first-class `QTY` (the **evaluated-value description**);
- **expression term** `me'o [mekso]` → a sign-sort `REF` (the **unevaluated** expression — the value/sign distinction, dictionary-confirmed by `li` vs `me'o`);
- **MOI converters** → number→relation `rel`s: `mei` (cardinality, x1 mass/set of n members x2), `moi[n]` (ordinal, x1 is n-th of x2 by rule x3), `si'e[n]` (portion), `cu'o[n]` (probability).

Internally a mekso term is a `MEX` node (operator + operands); operators are VUhU (`su'i`=+, `vu'u`=−, `pi'i`=×, `te'a`=^, …), and `nu'a [op]` converts an operator back to its **relation** (a selbri). Operands: numbers, `li`-quantities, lerfu variables (0.M), special numbers (`pi`=π, `te'o`=e, `ka'o`=i, …), `xi`-subscripts. Infix/forethought/RPN are surface orderings of the same `MEX` tree.

## 0.O Discourse structure

- **`.i` / NIhO** = discourse **juxtaposition** → a **`SEQ`** (truth-valueless; each member `FRM` keeps its own truth value and existential import). NIhO is a higher-level `SEQ` grouping (paragraph). The **`.ije`-series** are `FRM` connectives (0.L), live only between the two bridi they join (within a `SEQ` item). **`.i` is never a truth function** — so all multi-sentence text is a `SEQ`, not a conjunction.
- **Quotation, two registers.** **Structured** `lu…li'u` → a **nested `UTT`** (fully parsed content with its intrinsic force, with its **own speaker/audience roles**, and **referents reachable** from outside by shared id). Mention status comes from the containing quotation/sign edge, not from rewriting the nested utterance's force. **Opaque** `lo'u…le'u` / `zo` / `zoi` → a **`SGN kind=quotation`** (token sequence, no internal structure, **no reachable referents**). `zo` = single-word token; `zoi .X. … .X.` = foreign opaque text (the only way to quote non-Lojban / rafsi); `la'o .X. … .X.` = a **name from foreign text** (= `la me zoi`). **`la'e`/`lu'e`** = the reference↔referent shifts (`lae[]`/`lue[]`): the triad `zo .bab.` (word) / `la .bab.` (named thing) / `la'e zo .bab.` (the word's referent).
- **Topic-comment `zo'u`** = a topic `REF` linked to the comment by a **vague contextual `R[topic-of]`** — the topic's argument place in the comment is **genuinely underspecified** (exposed, not fabricated). Same `zo'u` as the 0.E prenex; the pre-`zo'u` string may carry both quantifier bindings and a topic. Multi-sentence scope via `tu'e…tu'u`.
- **TO/TOI, SEI/SE'U** = parentheticals → a nested `UTT force=parenthetical`, attached to the host by an **unordered `aside` edge** outside `FRM`, sharing the host's deictic ground by default. Its content, when it is several unconnected sentences, is a **`SEQ`** (truth-valueless); when the bracketed sentences *are* logically connected (`to Q1 .ije Q2 toi`) that is an ordinary `FRM` connective inside one `SEQ` item — the grouping there is **precedence only**. A non-logical expression spliced where a connective operand is expected **does not typecheck**; it is represented without imparting meaning, inference left to the reader.
- **FUhE…FUhO** = attitude scope: the bracketed span becomes a `DSP`'s `target` (0.J). **MAI / XI** = `MAI` utterance-ordinals (an ordinal **label on a `SEQ` item**); `XI` subscripts (a `subscript` annotation yielding a **distinct** global symbol — `da xi re` ≠ `da`). **BAhE** = contrastive emphasis → a `DSP family=emphasis` focus annotation (no truth effect).
- **SI / SA / SU / FAhO** = token-stream **erasure / end-of-input**, applied left-to-right as a **pre-semantic prelude** (like rafsi decomposition and the magic-word rules): already applied by the time the model exists, hence **absent** from the final structure; erasers do not operate inside `lo'u…le'u` (below the parse).

## 0.P Indexicals are utterance roles, not flavors

`mi`/`do`/`ti…`/`ko` are **not** a referent flavor. The `UTT` carries `speaker`/`audience`/deictic-ground **roles**, and an indexical denotes a **role of the specific (possibly nested/quoted) utterance the word lexically occurs in**:
- top level: `mi` → `u1.speaker`, `do` → `u1.audience`; `ti/ta/tu` → the utterance's deictic ground.
- inside a **structured quotation** (`lu…li'u`), the quoted text carries its **own** `UTT` node `u2` with its **own** speaker/audience, so an inner `mi` → `u2.speaker`. Whether `u2.speaker = u1.speaker` is an **explicit `du` claim, never implicit** — this is the case a flavor could not represent.
- `ko` = `u_n.audience` **plus** `force=command`. `mi'o`/`mi'a`/`ma'a` = `(speaker ⊕ audience ⊕ …)` composites of roles. `ra'o` = re-point an indexical from `u_old.speaker` to `u_new.speaker` (on a `go'i`-copy). `lo`/`le`/`la` keep `flavor`; bound `var`s and `PAR`s are unaffected.

An unstated speaker/audience still gets an explicit placeholder `REF` (an ordinary unspecified constant), so the roles are always present and referenceable, and their existence rides on the asserted `UTT` (0.K).

---

# Changelog (authoritative)

Entries marked **SUPERSEDED** are retained only to record the model's history; the live model is the union of the non-superseded entries, as consolidated in Part 0.

**Baseline (B1–B7), from the pre-encoding debates.** Eventualities are referents (the one relation-independent argument every predicate carries, abstracted by `nu` as `ka` abstracts a participant; tense/aspect are predicates *of it*). Holes are `PAR`s. No identity-binder/scope nodes. Sense = locutionary content + force on the utterance. Use-conditional content = `displayed` predications. Sumtcita = shared-eventuality predications with a link ρ. Tanru desugar.

- **C-1, C-3, C-5 — SUPERSEDED by C-G.** (Early gadri treatment: `lo`/`le` as veridical/non-veridical plural constants with plurality-mode *flavors* and a special "description" tag. Replaced wholesale.)
- **C-2 — SHARPENED to C-9.** (Early "outer quantifier = restricted variable".)
- **C-G (guskant plural-constant basis).** Gadri introduce fresh constants with typed descriptors, exactly as in Part 0.D. Veridical descriptors and the property inside a speaker description use restrictive bodies; `le` is structurally non-veridical through its incidental `skicu` body. Names, masses, and sets use their distinct descriptor kinds and sorts rather than a `flavor` field. Distributivity is not encoded by the bare constant. (Part 0.C–0.D.)
- **C-4 (implicit-value surfacing + utterance framing).** A bare predication `EV` is a generated-bound witness with unspecified time and no CAhA commitment unless one is explicit; it is *not* automatically present or actual. Every elided place is its own `zo'e` `REF`; every example is wrapped in the `UTT`/locution frame. (Part 0.B.)
- **C-8 (inner quantifier & constant laws).** Inner quantifier = `mei`-count, not logical; individual ≡ `pa mei`; `lo no broda` meaningless; constants carry existential import + the Skolem reading. (Part 0.C–0.E.)
- **C-9 (outer quantification).** Outer `PA` = a restricted bound **singular** variable (`PA da poi me …`), distributive by default; `su'oi`/`ro'oi` plural. (Part 0.E.)
- **C-12 (notation, later refined by C-22).** One object per line; uniform `KIND id : attr=val`; modes asserted/incidental/displayed/restrictive; every elided place its own `zo'e`; the `UTT`/`e0` frame always present. (Part 0.A–0.B.)
- **C-13 (existential import).** Import attaches to a predication-occurrence and propagates through `FRM` as that predication's truth value does; a bare `REF` is inert; `zi'o`/`PAR` exempt; a `var` is governed by its own quantifier; incidental/displayed carry import but project. *Refinement (CLL 16.8, corrected by C-30):* a `poi`-restriction on `forall`/`pluralForall` carries **projective** domain-nonemptiness import, recorded by `domainImport=projective`; restricted `exists`/`pluralExists`/`cardinality` derive their at-issue import classically from the restriction-as-conjunct reading, while `none` carries no import. A predication in a connective antecedent is not a quantifier restriction and carries no such domain import, as in the import-free "any". (Part 0.K, 0.E.)
- **C-14 (relative clauses & possession).** `poi` = restrictive (extension-fixing); `noi` = incidental; `voi` = restrictive but non-veridical (`skicu`-style); `pe`/`po`/`po'e` = restrictive `srana`/possession predications; `ne` = incidental; `goi` = shared id; `zi'e` = several clauses on one head; `vu'o` = a clause over a composite referent. (Part 0.A, companion ch. 8.)
- **C-15 (modals & tenses are sumtcita).** Modal = a typed `ModalArgument` on the host predication, with ρ internal when the tag gismu has an event place and contextual otherwise; tense = `EV` attributes; `FA` = argument reorder; `JAI` = a raising conversion; `KI` = sticky; sentence causals = event-relation formulas in sequence `connectionClaims`. (Part 0.G–0.H.)
- **C-16 (abstraction).** Direct abstraction-output referents, the `ce'u`/`ke'a` distinction, `du'u` factivity-belongs-to-the-verb, `se du'u` = the sentence-sign, `kau` focus, and `tu'a` raising. (Part 0.I.)
- **C-17 (indexicals as utterance roles).** Indexicals denote roles of the utterance they occur in; nested quotations are their own `UTT`s with their own roles; cross-level coreference is an explicit `du`; unstated speaker/audience get placeholder `REF`s whose existence rides on the asserted `UTT`. (Part 0.P.)
- **C-18 (lujvo place structure; amended).** A lujvo's relation is atomic, with `REL` metadata (`sourceWords`, `placeStructure`, and typed `expansion`/`rafsiBindings`) retaining the available decomposition and implicit direct-abstraction information; **tanru desugar in `FRM`, lujvo do not**. (Part 0.F.)
- **C-19 (attitudinals & evidentials).** Every UI/CAI indicator is a `DSP` (no truth value, never in `FRM`); pure emotion leaves the bridi asserted, propositional attitudes and hearsay/assumption evidentials subordinate it, holds-from-source evidentials assert-with-source-marking, `ca'e` is performative; `dai`/`pei`/`ge'e`/`bu'o` are the experiencer-shift/question/null/contour operators; insincerity = infelicity. (Part 0.J.)
- **C-20 (the connective system).** All logical connectives desugar to one `FRM` truth-functional structure, the locus fixing sharing/distribution; A/E/O/U + `na`/`nai`/`se` algebra; U's `inert` mode; connective questions = a `PAR` at the connective node; non-logical JOI builds composite referents. (Part 0.L.)
- **C-21 (negation's three kinds).** Contradictory `na`/`naku` (`FRM` `not`, positional, `naku` a boundary inverting quantifiers / forcing DeMorgan); scalar `na'e`/`no'e`/`to'e`/`je'a` (relation-level, positive); metalinguistic `na'i` (displayed tier, no truth value). (Part 0.K.)
- **C-22 (quantifier scope as `FRM` operator nodes — canonical reference-by-id).** Logical quantification takes scope as `EX`/`ALL`/`CARD`/`EXP`/`ALLP`/`NO` nodes that **reference a body formula by id** (never a literal subtree). The binding edge (type, scope, position relative to other binders and `not`) is the irreducible content global uniqueness does not supply; per-`PRD` conditionality follows automatically and is left unannotated. Subsumes C-8/C-9 and the Skolem story; identity/anaphora stay flat. (Part 0.E.)
- **C-23 (letterals; final).** A lerfu is a sign-sort `REF kind=letteral` with typed `letterals` units and preserved source words. Character use returns that sign; pro-sumti use resolves to an existing referent id without a public handle field; mekso-variable use is a typed `MEX` literal. (Part 0.M.)
- **C-24 (mekso).** Numbers denote values; the `MEX` operand language is self-contained, feeding the core via quantifier (`CARD`), value (`li`→`QTY`), expression (`me'o`→a sign-sort `REF`), and MOI relations; `li`/`me'o` is the value/sign split. (Part 0.N.)
- **C-25 (text structure).** `.i`/NIhO → `SEQ` (truth-valueless; bare `.i` uses `same-topic-continuation`, NIhO a tagged paragraph boundary); `.ije` → a connected `content` formula on the sequence; structured (`lu…li'u`, nested `UTT`, referents reachable) vs opaque (`lo'u`/`zo`/`zoi`, sealed sign-sort `REF`) quotation; `la'o`/`la'e`/`lu'e`; topic-comment `zo'u` = vague `R[topic-of]`; TO/SEI = `aside` edge; FUhE/BAhE = `DSP`; MAI/XI = annotations; SI/SA/SU/FAhO = pre-semantic token editing. (Part 0.O.)
- **C-NL (non-logical / discourse attachment).** Parentheticals (`to…toi`, `sei…se'u`) are nested `UTT force=parenthetical` on an unordered `aside` edge, sharing the host's deictic ground; their content, when several unconnected sentences, is a `SEQ` (**truth-valueless by construction**, each member keeping its own truth value and import); a `SEQ` member may itself be a logically connected `FRM` (grouping = precedence only); `.i` is discourse juxtaposition, never a truth function. (Part 0.O.)
- **C-26 (tersmu / Lean-prelude cross-check).** Validated the model against jbotci `tersmu`'s Lean prelude (the prior object model, read directly). Three refinements. **(A)** Tanru now use one **uniform** schema — tertau asserted + seltau reified as a kind `⟨ka ce'u S⟩` linked by the vague `R`, asserting **neither** `S(x)` **nor** a concrete seltau referent; the reading is the resolution of `R`, and only the tertau's eventuality is introduced. (The prelude's `ofKind` is likewise an *uninterpreted* modifier that closes the seltau's extra places; the prior two-sub-case form over-committed by fabricating `S(x)`/`S(y)` and by proliferating events.) **(B)** The `REF` sort list gains `Proposition` (`du'u`) and `TruthValue` (`jei`) so abstraction outputs are typed for place-structure checking (the prelude keeps distinct `Predication`/`TruthValueEntity`/`ConceptEntity`/`AmountEntity`/… wrappers), and `ka`'s arity is fixed as its `ce'u` count. **(C)** `gi'e` shares only an **explicit** x1 — an omitted x1 is each tail's own `zo'e` (CLL 14.57–14.58) — and sumti connection shares overt non-connected arguments while keeping omitted non-connected places branch-local. **Corroborated without change:** the `UTT` speaker/addressee frame (`Utterance := Entities → Entities → Prop`), reified eventualities, claim-by-omission, `le` as non-veridical description, the `gunma`/`selcmi` mass/set reductions (the prelude cites the same BPFK `lo gunma/selcmi be …`), the four distinct causals (`isPhysicallyCausedBy`/`isJustifiedBy`/`isMotivatedBy`/`isLogicallyEntailedBy`), `caha` (`isActualized`/`isCapableOf`/…), `se du'u` as a sentence-sign (`TextEntity`), **lujvo as lexical-atomic with a retained veljvo witness** (`asLujvo`, exactly C-18), structured quotation as a nested speaker-relative utterance, and — independently reproduced — the `.e`-vs-`gi'e` sharing contrast. **Deliberate divergences kept per mandate:** xorlo `lo` (prelude uses the pre-xorlo existential conjunct), the propositional-attitude vs pure-emotion split (prelude treats both as a bare `Attitude` label, leaving the bridi asserted — a real value-add we keep), existential import of restricted universals (CLL 16.8; prelude uses bare `∀`), and gismu-derived modal `ρ` exposing places (prelude uses curated `is…By` relations). Current names use a typed name descriptor rather than a separate `cmene` namer predication and word-sign. (Parts 0.A, 0.F, 0.I, 0.L.)
- **C-27 (jbotci CLL review pass, 2026-06-23).** A CLL-wide review of `jbotci gentufa`/`tersmu` (chapters 9, 10, 11, 14 + pilots) adopted a batch of model amendments and, at the time, recorded implementation divergences. Those listed divergences have since been resolved; the current audit section in [`semantic-model-spec.md`](semantic-model-spec.md) is authoritative. Highlights, all consistent with Part 0: tenses/aspect/`ROI`/`ki` are realized as the eventuality attributes 0.G already prescribes (stacked aspect gets an ordered `intervalModifiers` stack; `ROI` counts become first-class `quantity` objects; `ki` gets a `sticky` flag); the 0.L connective algebra is realized **structurally** rather than left in surface text (whether-or-not marks the `inert` operand; `se` exchanges operands; **negated logical connectives canonicalize to the base-vowel operator with a `not`-wrapped operand at every locus — the `na ja`→`implies` shortcut is dropped**; runs nest binary, never flattening across distinct operators); non-logical statement connectives (`.i joi`/`.i ce'o`) get a truth-valueless `nonlogicalConnection` on the `SEQ`; connected operands carry `force=subordinated`; tanru keep the 0.F desugaring but gain a typed `tanruLink` sidecar replacing the untyped `R[tanru:…]` string; `lo'e`/`le'e` bodies become non-veridical (kind/genericity structure deliberately **not** added — 0.D keeps that gap exposed); `fa'u` gets a declarative `respectivelyDistribution` node; `vau`-shared ids become normative. Doc-only items: `fi'o se pilno` place-numbering, `quotation.mode` as a category, and the `abstractionAbout` descriptor kind were already correct — the consumer primer was corrected instead. (Parts 0.D, 0.F, 0.G, 0.H, 0.I, 0.L, 0.O.)
- **C-28 (mekso connective implementation follow-up).** Logical mekso operand
  connectives used as sumti quantifiers now lift to formula-level connectives
  over the resulting quantifier scopes, and logical mekso operator connectives
  inside `li ... du ...` identity claims lift to formula-level connectives over
  the two substituted identity formulas.  This realizes CLL 14.149–14.152
  without inventing opaque operator strings or placeholder quantities. (Part
  0.N.)
- **C-29 (`fa'u` stream implementation follow-up).** `fa'u` keeps its
  non-logical respectively composite for ordinary argument formation, but
  truth-conditional parallelism is represented by a `respectivelyDistribution`
  formula whose `streams` bind `parameter.role="respectiveSlot"` placeholders.
  Quantified witness streams carry their restriction and source quantity
  (CLL 14.124), while termset streams may carry complete branch formulas so
  modal/tag content stays paired with the correct index (CLL 14.133). (Part
  0.L.)
- **C-30 (projective restricted-universal domain import, #279).** Adopted the
  classical-divergence criterion and replaced 0.E's incorrect at-issue
  `and(ALL, EX)` encoding with `domainImport=projective` on restricted
  `ALL`/`ALLP` nodes. The marker records the one fact the structural graph
  cannot express faithfully: domain nonemptiness survives `not`. The
  `naku`-duality proof in 0.E shows why an ordinary conjunct is invalid;
  existential/cardinality restrictions remain unannotated because their import
  is already classical, and `NO` remains non-importing. (Parts 0.E, 0.K.)
- **C-31 (typed constant scope dependence, #352).** Every constant carries an
  explicit `scopeDependence`: `fixed` at binder-free introduction sites, or
  `underspecified` with the mechanically derived nonempty `mayDependOn` set.
  This exposes the existing C-8/C-22 Skolem reading without asserting actual
  dependence. Claims-ledger denotation lines render the state per line and stay
  in the projected tier because dependence is orthogonal to commitment status.
  (Parts 0.C, 0.E.)
- **C-32 (typed generated-event binding, #353).** Eventualities distinguish
  generated-bound witnesses from referential denotations. Every generated event
  is bound exactly once on the lowest dominating formula, or on the lowest
  containing sequence when formula roots have no LCA; referential events never
  occur on that edge. The binding supplies existential scope only, and generated
  events no longer project denotation lines or carry `scopeDependence`. (Parts
  0.A, 0.B, 0.G, 0.H.)
