# Lojban → Discourse Object Model: Worked Encodings (CLL 5–19)

*Companion to `semantic-model-design.md`, retained as design history. Every
example uses a compact reference-by-id notation: one object per line and
compound formulas reference sub-formulas by id. CLL is authoritative except for
the gadri (xorlo/guskant).*

> **Status (jbotci#869).** These encodings target the object model of the
> `jbotci tersmu` implementation that was retired from this repository; they
> describe no live jbotci surface. The successor semantic specification lives
> in the separate [smusni](https://github.com/int19h/smusni) project.

**Reading the notation.** `KIND id : attr=val, …` is a compact algebra, not a
second JSON schema. In the retired public graph:

- `UTT`, `SEQ`, `FRM`, `PRD`, `PAR`, `DSP`, `MEX`, and `QTY` abbreviate the
  public `utterance`, `sequence`, `formula`, `predication`, `parameter`,
  `displayedContent`, `mathExpression`, and `quantity` object types.
- `SGN` abbreviates a `type:"referent"`, `sort:"sign"` object; `sign` is not a
  public object `type`.

- `REF ... kind=const, flavor=X` abbreviates a `type:"referent"`,
  `category:"constant"` object with derived `scopeDependence` and the
  corresponding typed `descriptor`/`sort`; it does not name literal JSON
  fields.
- `+cmene-clause(x, name)` is legacy notation for `x`'s typed `name`
  descriptor. The retired graph did not add a separate `cmene` predication or
  word-sign merely to encode a cmevla name.
- `EV` is an eventuality-sort `type:"referent"`. A bare predication event is
  `denotation:"generated-bound"`, has unspecified time and actuality, and is
  listed in `boundEventualities` on its formula/sequence owner. `tense`/`caha`
  below are compact spellings for the typed time/`actuality` fields.
- `RFY` is historical shorthand for fields on a **direct abstraction-output
  referent**, not a public wrapper object. Non-event outputs carry `body`;
  eventuality outputs carry `content`; parameters and embedded questions stay
  on that output. The algebra's `abstracted=[...]` list names those output
  parameters or the eventuality denoted by the output; it is not a public JSON
  field.
- Veridical descriptor predications and the property relation inside a speaker
  description use `mode=restrictive`. A `speakerDescription` descriptor itself
  contains an incidental `skicu` predication, and genuine side claims such as
  `noi` are incidental too.

The standing frame
```
UTT u1 : force=assert, content=f1, ev=e0, speaker=a1, audience=a2, deictic-ground={time:n0, place:h0}
REF a1 : category=indexical, indexical=speaker, sort=entity
REF a2 : category=indexical, indexical=audience, sort=entity
EV  n0 : denotation=referential, category=indexical, indexical=now, sort=eventuality
REF h0 : category=indexical, indexical=here, sort=entity
EV  e0 : denotation=referential, category=constant, sort=eventuality/locution, actuality=actual
```
is present in every example; after the first occurrence per chapter it is
`⟨frame u1/e0/a1/a2⟩`. A bare `EV en` means `time=unspecified,
actuality=unspecified`. Elided places are explicit `zo'e` referents.

---
## Chapter 5 — selbri

**5.7 `la .djan. barda nanla`** — "John is a big boy" (intersective tanru: x is both big and a boy).
```
⟨frame u1/e0/a1/a2⟩
REF d1 : category=constant, sort=entity,
         descriptor={kind:name, word:la, speaker:a1, name:"djan"}
EV  eT : tense=?, caha=?
PRD pT : rel=nanla, ev=eT, args=[d1, z2, z3], mode=asserted         -- tertau (primary): d1 is a boy; supplies the one event
EV  eK : tense=?, caha=?
REF k1 : category=constant, sort=relation,
         body=⟨barda(eK; ce'u, z4, z5) restrictive⟩, parameters=[ce'u], arity=1
PRD pR : rel=tanru, tanruLink={head:pT, modifier:k1, label:barda-nanla}, args=[d1, k1], mode=asserted   -- vague tanru link: d1 stands to the "big" kind
REF z2 : kind=const, flavor=zo'e, sort=Obj
REF z3 : kind=const, flavor=zo'e, sort=Obj
REF z4 : kind=const, flavor=zo'e, sort=Obj
REF z5 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : and(pT, pR)
```
`la djan` selects **this one** referent `d1` — not everyone so named — and
records the name in its typed descriptor rather than emitting a separate
`cmene` predication. Per 0.F's uniform schema the seltau `barda` is the
relation referent `k1`, linked by the vague `R`; the **intersective** reading
resolves `R` to instantiation (unfolding to `barda(d1)`), so the structure
records the link rather than asserting a separate `barda(d1)` conjunct.

**5.13 `ta cinfo kerfa`** — "that is a lion-mane" (**asymmetrical** tanru: the seltau is *not* predicated of x).
```
⟨frame u1/e0/a1/a2⟩
REF t1 : category=indexical, indexical=medialDemonstrative, sort=entity  -- ta: the demonstrated mane
EV  eT : tense=?, caha=?
PRD pT : rel=kerfa, ev=eT, args=[t1, z1, z2], mode=asserted         -- tertau (primary): t1 is a mane (of body z1); the one event
EV  eK : tense=?, caha=?
REF k1 : category=constant, sort=relation,
         body=⟨cinfo(eK; ce'u, z3) restrictive⟩, parameters=[ce'u], arity=1
PRD pR : rel=tanru, tanruLink={head:pT, modifier:k1, label:cinfo-kerfa}, args=[t1, k1], mode=asserted       -- vague link: t1 stands to the "lion" kind
REF z1 : kind=const, flavor=zo'e, sort=Obj
REF z2 : kind=const, flavor=zo'e, sort=Obj
REF z3 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : and(pT, pR)
```
Under 0.F's uniform schema **no lion referent is introduced at all**: the seltau `cinfo` is the reified kind `k1`, and the typed `tanru` link connects the mane `t1` to that kind. Nothing asserts a lion exists — the selbri alone doesn't entail one — so neither the intersective `cinfo(t1)` (the mane is not a lion) nor a concrete `cinfo(y1)` is claimed; the lion-relationship lives entirely in the unresolved `R`. (Same shape handles `junla dadysli`, `rokci cinfo`, etc. — the reading is just which `R`.)

**Deltas for the other chapter-5 constructs** (each changes only a little):
- **Plain `remna nakni` ("man")** — retains the same uniform tanru shape:
  asserted `nakni` head plus a direct `remna` relation modifier and typed
  `tanruLink`. Its familiar intersective reading is a contextual resolution of
  that link, not a second asserted conjunct fabricated by the model. Explicit
  logical selbri connection uses formula connectives instead.
- **`joi` (5.58 `blanu joi xunre`)** — a `gunma`-of-properties composite `(blanu ⊕ xunre)` fed to a colour predication (collective), not an `and` in `f1`.
- **`be` (5.64 `ti xamgu be do bei mi`)** — the seltau `PRD` has filled args: `xamgu(eS; ti, do, mi) mode=asserted`, no `zo'e` in x2/x3.
- **`co` (5.79)** — identical bag to the un-inverted tanru; word order only.
- **`SE` (5.110 `do se prami mi`)** — the predication keeps
  `relation=prami`; conversion places `mi` under root x1 and `do` under root
  x2, so the graph asserts `prami(mi, do)` while preserving canonical
  root-relation place keys.
- **`me` (5.99 `la .baltazar. cu me le ci nolraitru`)** —
  `me(e1; B, K) mode=asserted`, with `K` the `le ci nolraitru` constant. Its
  descriptor points at an exact-three quantity and carries the `LE-clause`.
- **scalar (5.117 `… na'e cadzu klama …`)** — the `cadzu` modifier
  predication retains `relation=cadzu` and carries typed
  `scalarNegation={kind:otherThan, introducedBy:na'e, ...}`; the tanru link
  then relates that scalar-modified property to the asserted `klama` head.

## Chapter 6 — sumti

**The canonical claim-by-omission — `lo botpi cu xunre`** ("a bottle is red"). `botpi` = x1 is a bottle for contents x2, of material x3, with **lid x4**; so a lid is asserted to exist.
```
⟨frame u1/e0/a1/a2⟩
REF b1 : category=constant, sort=entity,
         descriptor={kind:veridicalDescription, word:lo, speaker:a1, body:fb}
PRD pb : rel=botpi, args=[b1, z1, z2, z3], mode=restrictive         -- descriptor body fb; z3 = the LID
REF z1 : kind=const, flavor=zo'e, sort=Obj                          -- contents
REF z2 : kind=const, flavor=zo'e, sort=Obj                          -- material
REF z3 : kind=const, flavor=zo'e, sort=Obj                          -- LID: its existence is asserted (import via pb)
EV  e1 : tense=?, caha=?
PRD p1 : rel=xunre, ev=e1, args=[b1, z4], mode=asserted
REF z4 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : p1
```
Each elided place is its own `zo'e` `REF`; `z3` (the lid) occurs in the
restrictive descriptor body. To remove the lid place you would write `zi'o` in
x4 (no `REF`).

**6.6 `le zarci cu barda`** — full `le` expansion shown once, then abbreviated.
```
⟨frame u1/e0/a1/a2⟩
REF s1 : category=constant, sort=entity,
         descriptor={kind:speakerDescription, word:le, speaker:a1, body=f3}
REF k1 : category=constant, sort=relation, body=fk, parameters=[c1], arity=1
PAR c1 : sort=entity, role=propertySlot
PRD pk : rel=zarci, args=[c1, z1, z2], mode=restrictive              -- property body: ce'u is a market
REF z1 : kind=const, flavor=zo'e, sort=Obj
REF z2 : kind=const, flavor=zo'e, sort=Obj
FRM fk : atom(pk)
PRD p3 : rel=skicu, args=[a1, s1, a2, k1], mode=incidental
FRM f3 : atom(p3)
EV  e1 : tense=?, caha=?
PRD p1 : rel=barda, ev=e1, args=[s1, z3, z4], mode=asserted
REF z3 : kind=const, flavor=zo'e, sort=Obj
REF z4 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : p1
```
The only at-issue claim is `barda(s1)`. The `speakerDescription` descriptor
carries the incidental `skicu` predication, whose x4 is the direct
relation-sort property `k1`; `k1`'s own `zarci` body is restrictive. Thus `le`
can be false of its noun. Henceforth `LE-clause(x, broda)` abbreviates this
two-level descriptor shape.

**6.9 `lo mlatu cu gerku`** — false, by shared referent identity (not by tier).
```
⟨frame u1/e0/a1/a2⟩
REF c1 : category=constant, sort=entity,
         descriptor={kind:veridicalDescription, word:lo, speaker:a1, body:fm}
PRD pm : rel=mlatu, args=[c1, z1], mode=restrictive
REF z1 : kind=const, flavor=zo'e, sort=Obj
EV  e1 : tense=?, caha=?
PRD p1 : rel=gerku, ev=e1, args=[c1, z2], mode=asserted
REF z2 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : p1
```
`p1` is unsatisfiable because the referent selected by the veridical `mlatu`
descriptor must also satisfy `gerku`; no animal qualifies. `le mlatu cu gerku`
(6.7) is *not* contradictory: its descriptor is `speakerDescription`, so a dog
the speaker describes as a cat satisfies it.

**6.17 `lei prenu cu bevri le pipno`** — the mass carries the piano (collective; contradictory skin-colours tolerated).
```
⟨frame u1/e0/a1/a2⟩
REF x1 : category=constant, sort=entity,
         descriptor={kind:speakerDescription, word:le, speaker:a1, body:fp}
REF m1 : category=constant, sort=mass,
         descriptor={kind:speakerMassDescription, word:lei, speaker:a1, body:fg}
PRD pg : rel=gunma, args=[m1, x1], mode=restrictive
FRM fg : atom(pg)
REF K1 : kind=const, flavor=le, sort=Obj ; +LE-clause(K1, pipno)
EV  e1 : tense=?, caha=?
PRD p1 : rel=bevri, ev=e1, args=[m1, K1, z1, z2, z3], mode=asserted   -- the MASS carries K1: collective
REF z1 : kind=const, flavor=zo'e, sort=Obj
REF z2 : kind=const, flavor=zo'e, sort=Obj
REF z3 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : p1
```
The carrier is the mass-sort referent `m1`, so `bevri` does **not** distribute
to each person — exactly CLL's "carried it jointly". **6.18
`loi cinfo cu xabju le fi'ortu'a`** uses `veridicalMassDescription` with a
veridical `cinfo` member body. The mass is a direct mass-sort referent whose
restrictive descriptor body contains its `gunma` relation to that member
referent, the same structural reduction expressed by `lo gunma be lo cinfo`.

**6.24 `lo'i ratcu cu barda`** — the *set* is large.
```
⟨frame u1/e0/a1/a2⟩
REF x1 : category=constant, sort=entity,
         descriptor={kind:veridicalDescription, word:lo, speaker:a1, body:fr}
REF S1 : category=constant, sort=set,
         descriptor={kind:veridicalSetDescription, word:lo'i, speaker:a1, body:fs}
PRD ps : rel=selcmi, args=[S1, x1], mode=restrictive
FRM fs : atom(ps)
EV  e1 : tense=?, caha=?
PRD p1 : rel=barda, ev=e1, args=[S1, z2, z3], mode=asserted           -- the SET is large
REF z2 : kind=const, flavor=zo'e, sort=Obj
REF z3 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : p1
```
`barda` predicates of the set object `S1`; `bunre(S1)` would be a sortal
mismatch the model exposes rather than forbids. **6.26
`lo'e cinfo cu xabju le fi'ortu'a`** (typical lion) uses a `typicalDescription`
referent whose `cinfo` body is non-veridical; it is the intensional generic.

**6.31 `re do cadzu le bisli`** — "two of you walk on the ice" (outer quantifier ⇒ restricted distributive variable; reference-by-id).
```
⟨frame u1/e0/a1/a2⟩
REF v1 : kind=var, sort=Obj                                          -- re da
PRD pm : rel=memberOf, args=[v1, a2], mode=restrictive                -- v1 is selected from the audience
REF I1 : kind=const, flavor=le, sort=Obj ; +LE-clause(I1, bisli)
EV  e1 : tense=?, caha=?
PRD p1 : rel=cadzu, ev=e1, args=[v1, I1, z1], mode=asserted
REF z1 : kind=const, flavor=zo'e, sort=Obj
FRM fr : atom(pm)
QTY q2 : form=exact, value={integer:2}, scale=count
FRM f1 : cardinality(variable=v1, restriction=fr, body=atom(p1), quantity=q2)
```
`re do` uses an exact-two quantity and a cardinality formula whose variable is
restricted to membership in the audience, distributively. **6.39
`re le ci gerku cu blabi`** (inner + outer) gives the `speakerDescription`
constant `G` an exact-three `descriptor.quantity`, then gives a fresh variable
an exact-two cardinality scope with `memberOf(v,G)` as its restriction and
`blabi(v)` as its body. **6.44 `ci gerku cu blabi`** (indefinite) has no inner
constant: its exact-three cardinality variable is restricted directly by
`gerku(v,…)` and has `blabi(v)` as its body.

**6.53 / 6.54 LAhE — `mi viska la'e lu le xunre cmaxirma li'u`** and `mi pu cusku lu'e le vi cukta`.
```
⟨frame u1/e0/a1/a2⟩
SGN Q1 : kind=quotation, quotation={mode:parsed, utterance:u2, text:"lu le xunre cmaxirma li'u"}
UTT u2 : force=mention, content=fq, ev=eq, speaker=b1, audience=b2   -- parsed quoted title
… (fq = the parsed title's content; its roles b1/b2 its own) …
EV  e1 : tense=?, caha=?
PRD p1 : rel=viska, ev=e1, args=[a1, lae[Q1], z1], mode=asserted      -- la'e Q1 = the book the sign denotes
REF z1 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : p1
```
`lae[Q1]` = `lo se sinxa be Q1` (the book). `lu'e` is the converse: `cusku(e1[tense=pu]; a1, lue[K], z)` with `K = le vi cukta`, `lue[K]` a sign for the book (its title). **6.56 `mi troci tu'a le vorme`**: `troci(e1; a1, g, z)` with `RFY g : kind=su'u, body=⟨R(eR; V, z')⟩, abstracted=[eR]`, `V = le vorme` (a raised `co'e` abstraction). **6.57 `… lu'a ri cmalu`**: `ri` is the **same id** as the last referent (the set `S1`); `lu'a[S1]` = its members. **`na'ebo le gerku` (6.60)**: `viska(e1; a1, ⟨na'e: G⟩, z)` — a scalar-other-than over the constant `G = le gerku`.

## Chapter 7 — pro-sumti and pro-bridi

Chapter 7 is the **payoff of desugaring**: anaphora, assignment, reflexives, and assigned pro-bridi all reduce to **reuse of an existing node's id**. There is no anaphora machinery.

**7.32 `mi prami mi`** — reflexive by repetition; `mi`→`u1.speaker` (C-17), one node twice.
```
⟨frame u1/e0/a1/a2⟩
EV  e1 : tense=?, caha=?
PRD p1 : rel=prami, ev=e1, args=[a1, a1], mode=asserted              -- both places = u1.speaker
FRM f1 : p1
```

**7.33 `la .djan. viska le tricu .i ri se jadni le ri jimca`** — all `ri`s collapse to one id.
```
⟨frame u1/e0/a1/a2⟩
REF j1 : kind=const, flavor=la, sort=Obj ; +cmene-clause(j1, "djan")
REF Tr : kind=const, flavor=le, sort=Obj ; +LE-clause(Tr, tricu)
EV  e1 : tense=?, caha=?
PRD p1 : rel=viska, ev=e1, args=[j1, Tr, z1], mode=asserted
REF z1 : kind=const, flavor=zo'e, sort=Obj
REF Br : kind=const, flavor=le, sort=Obj ; +LE-clause(Br, jimca)    -- le ri jimca: the branches OF Tr
EV  ej : tense=?, caha=?
PRD pj : rel=jimca, ev=ej, args=[Br, Tr, z2], mode=restrictive       -- descriptor body; ri = Tr
REF z2 : kind=const, flavor=zo'e, sort=Obj
EV  e2 : tense=?, caha=?
PRD p2 : rel=se:jadni, ev=e2, args=[Tr, Br], mode=asserted           -- Tr (ri) is adorned by Br
FRM f1 : p1
FRM f2 : p2
SEQ s  : items=[u1, u2], relation=same-topic-continuation            -- two sentences = SEQ, not a conjunction
```

**7.38 / 7.39 — `go'i` copies a bridi; indexicals re-resolve per speaker (the C-17 payoff).** `mi klama le zarci .i do go'i`:
```
⟨frame u1/e0/a1/a2⟩
REF Z : kind=const, flavor=le, sort=Obj ; +LE-clause(Z, zarci)
EV  e1 : tense=?, caha=?
PRD p1 : rel=klama, ev=e1, args=[a1, Z, z1, z2, z3], mode=asserted    -- mi = u1.speaker
EV  e2 : tense=?, caha=?
PRD p2 : rel=klama, ev=e2, args=[a2, Z, z4, z5, z6], mode=asserted    -- explicit Z is replayed; omitted places are fresh
REF z1 : kind=const, flavor=zo'e, sort=Obj
REF z2 : kind=const, flavor=zo'e, sort=Obj
REF z3 : kind=const, flavor=zo'e, sort=Obj
REF z4 : kind=const, flavor=zo'e, sort=Obj
REF z5 : kind=const, flavor=zo'e, sort=Obj
REF z6 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : p1
FRM f2 : p2
SEQ s  : items=[u1, u2], relation=same-topic-continuation
```
`go'i` replays the explicit destination by id but allocates fresh elided
origin, route, and means referents for the new predication.
In 7.39 (`A: mi ba klama …` / `B: mi nelci le si'o mi go'i`), B's explicit `mi` is needed because A's `mi`=`uA.speaker` ≠ B's `mi`=`uB.speaker`; **`ra'o`** on the copy re-points the *copied* inner `mi` from `uA.speaker` to `uB.speaker` — exactly C-17's re-point operation. **7.48** (anaphora across quotations): `la .alis. cusku lu mi go'i li'u` — the `go'i` copies the bridi of the *quoted* `lu mi klama le zarci li'u` (an earlier nested `UTT`), and its `mi` re-resolves to Alice's quoted-utterance speaker; cross-quotation reference stays **within the quoted-utterance stream**, never reaching the narrative outside.

**Other constructs (all shared-id / one-node):**
- **`ma` (7.9)** `ma klama le zarci` → a `question` with
  `kind=argument`, `domain=entity`, and an answer slot whose parameter has
  `sort=entity, role=argumentQuestion`; the question body contains
  `klama(e1; q1, Z, zo'e…)`. **`mo`** uses `domain=relation` and a
  `relationQuestion` parameter in relation position.
- **`ko'a`/`goi` (7.5/7.36)** `… ri goi ko'a blanu` → `goi` binds the handle `ko'a` to the last referent's id (shared id); no new node.
- **`vo'a` (7.8)** `mi prami vo'a` → `prami(e1; a1, a1)` (vo'a = this bridi's x1 = `a1`).
- **`da`/`bu'a` (7.12)** `da poi gerku cu klama` → `REF X : kind=var`, `f1 : EX(X, and(gerku(X,…), klama(X,…)))`; `bu'a` = `REF B : kind=var, sort=Rel` in `rel` position, bound by an `EX` over relations.
- **`du` (7.14)** `do du la .djan.` → `du(e1; a2, j1) mode=asserted` (claimed identity, definable as mutual `me`).
- **`co'e`/`zo'e` (7.7)** → a free `R` in `rel` position / a `zo'e` `REF`. **`ko`** → `u1.audience` + `force=command` on the `UTT`. **`mi'o`/`ma'a`** → `(a1 ⊕ a2 ⊕ …)` role composites. **`di'u`/`la'e di'u` (7.4)** → a `REF` whose value is the prior `UTT` node / `lae[that UTT]` (its subject-matter) — the utterance-as-referent, cycle-capable.

## Chapter 8 — relative clauses & possession

**8.12 vs 8.13 — `le gerku poi blanu cu barda`** ("the dog *which is* blue is large") vs `noi` (incidental). Restrictive narrows the referent.
```
⟨frame u1/e0/a1/a2⟩
REF x1 : kind=const, flavor=le, sort=Obj ; +LE-clause(x1, gerku)
EV  eb : tense=?, caha=?
PRD pb : rel=blanu, ev=eb, args=[x1, z1], mode=restrictive           -- poi: x1 restricted to the blue one(s)
REF z1 : kind=const, flavor=zo'e, sort=Obj
EV  e1 : tense=?, caha=?
PRD p1 : rel=barda, ev=e1, args=[x1, z2, z3], mode=asserted
REF z2 : kind=const, flavor=zo'e, sort=Obj
REF z3 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : p1                                                          -- pb is NOT in f1: it co-fixes x1, and projects
```
`pb` is referent-fixing (it projects past `naku`: "the blue dog is not large" keeps the dog blue), so it is **not** a conjunct under `f1`'s operators. The `noi` version flips `pb` to `mode=incidental` — `x1` is the dog(s) and there is a *side-claim* they are blue (no narrowing). A restrictive `poi` here ≈ the tanru `le blanu gerku` (6.16/ch.5), but `poi` is sharper (no vague tanru `R`).

**8.7 possession — `le karce pe mi`** ("my car", loose association via `srana`).
```
⟨frame u1/e0/a1/a2⟩
REF c1 : kind=const, flavor=le, sort=Obj ; +LE-clause(c1, karce)
EV  es : tense=?, caha=?
PRD ps : rel=srana, ev=es, args=[c1, a1], mode=restrictive           -- pe: c1 pertains to me (= u1.speaker)
FRM (ps attaches to c1; the host clause supplies f1)
```
`po` (specific) → a stronger possession relation; `po'e` (inalienable) → an intrinsic part-relation `R[part-of]`; `ne` (incidental) → `ps` with `mode=incidental`. **`voi` (8.5)** `le voi blanu` (non-veridical restrictive) → `skicu(a1, x1, a2, ⟨ka ce'u blanu⟩) mode=restrictive` (restricted to what the speaker *describes* as blue). **`goi` (8.x)** binds a `ko'a` handle to the head's id (shared id). **`zi'e` (8.4)** → several clause-`PRD`s on one head (each with its own mode). **`vu'o` (8.8)** → the clause attaches to a composite `(c1 ⊕ …)` referent rather than to `c1` alone.

## Chapter 9 — adjuncts (sumtcita = shared-eventuality predications)

**9.28 `mi viska do sepi'o le zunle kanla`** — "I see you with-tool my left eye" (`sepi'o`←`pilno`, ρ **internal** via x3).
```
⟨frame u1/e0/a1/a2⟩
REF k1 : kind=const, flavor=le, sort=Obj ; +LE-clause(k1, [zunle ⋗ kanla])   -- the left eye (tanru)
EV  e1 : tense=?, caha=?
PRD p1 : rel=viska, ev=e1, args=[a1, a2, z1], mode=asserted, adjuncts=[m1]  -- I see you
ADJUNCT m1 : relation=pilno, introducedBy="se pi'o",
             arguments=[z_agent, k1, e1]                                     -- root places: x2 tool, x3 purpose event
REF z1 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : atom(p1)
```
The seeing event `e1` fills `pilno`'s x3 (purpose) — that *is* the ρ-link.
The adjunct is a typed `Adjunct` on `p1`, with root-relation place keys;
it is not emitted as a second free-standing predication or formula conjunct.

**9.30 `mi cadzu seka'a la .bratfyd.`** — "I walk with-destination Bradford" (`ka'a`←`klama`, SE selects the destination place).
```
⟨frame u1/e0/a1/a2⟩
REF B : kind=const, flavor=la, sort=Obj ; +cmene-clause(B, "bratfyd")
EV  e1 : tense=?, caha=?
PRD p1 : rel=cadzu, ev=e1, args=[a1, z1], mode=asserted                       -- I walk (surface z1)
PRD pk : rel=klama, ev=ek, args=[a1, B, z2, z3, z4], mode=asserted            -- seka'a: same motion, destination B
EV  ek : tense=?, caha=?
REF z1 : kind=const, flavor=zo'e, sort=Obj
REF z2 : kind=const, flavor=zo'e, sort=Obj
REF z3 : kind=const, flavor=zo'e, sort=Obj
REF z4 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : and(p1, pk)
```
The `klama` predication shares the agent `a1` and (by `R`) the same motion as `cadzu`; SE has put `B` in `klama`'s x2.

**`fi'o kanla` (external ρ) and `do'e` (R alone).** `kanla` has no event
place, so its `Adjunct` retains the filled `kanla` arguments and an
explicit external contextual link to the host event. `do'e` (9.34,
`lo nanmu be do'e le berti`) retains only that vague contextual adjunct relation,
with no tag gismu — fully vague "of".

**Causal sentence connection (9.7) — `ko'a broda .i ri'a bo ko'e brode`** (`ri'a`←`rinka`):
```
⟨frame u1/e0/a1/a2⟩  -- (ko'a, ko'e resolved to referents)
PRD pA : rel=broda, ev=eA, args=[…], mode=asserted ;  FRM fA : pA
PRD pB : rel=brode, ev=eB, args=[…], mode=asserted ;  FRM fB : pB
PRD pc : rel=rinka, ev=ec, args=[eB, eA, z1], mode=asserted    -- following event eB causes preceding effect eA
REF z1 : kind=const, flavor=zo'e, sort=Obj
FRM fc : atom(pc), boundEventualities=[ec]
SEQ s : items=[uA,uB], relation=same-topic-continuation,
        connectionClaims=[fc], nonlogicalConnection={operator:"nonlogical:ri'a bo", ...},
        boundEventualities=[eA,eB]
```
CLL 9.7 makes the first bridi the effect (root x2) and the following bridi the
cause (root x1). The relation claim is attached to the sequence rather than
conjoined into either utterance's content. (`mu'i`→`mukti`, `ni'i`→`nibli`,
`ki'u`→`krinu`, each a relation between the two events.) **`JAI` (9.12)**
`mi jai rinka le nu …` uses conversion while retaining the root `rinka`
relation and canonical argument keys (`jai gau` raises the agent). **Adjunct
negation (9.13)** is recorded on the adjunct structure. **`KI` (9.14)** sets an
adjunct as a sticky default copied onto later predications.

## Chapter 10 — tenses (typed eventuality fields; no numbered places)

**10.4 `mi pu klama le zarci`** — "I went to the store" (past).
```
⟨frame u1/e0/a1/a2⟩
REF s1 : kind=const, flavor=le, sort=Obj ; +LE-clause(s1, zarci)
EV  e1 : time={relation:before, anchor:n0}                        -- pu: τ(e1) < now; ZI distance unspecified
PRD p1 : rel=klama, ev=e1, args=[a1, s1, z1, z2, z3], mode=asserted
REF z1 : kind=const, flavor=zo'e, sort=Obj
REF z2 : kind=const, flavor=zo'e, sort=Obj
REF z3 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : p1
```

**10.50 `mi ze'epu noroi klama le zarci`** — "I have never gone to the store" (interval extent + frequency, combined on one `EV`).
```
⟨frame u1/e0/a1/a2⟩
REF s1 : kind=const, flavor=le, sort=Obj ; +LE-clause(s1, zarci)
EV  e1 : time={relation:before, anchor:n0}, timeInterval={extent:whole},
         recurrence=[{kind:occurrenceCount, quantity:q0}]         -- q0 is exactly 0, introduced by noroi
PRD p1 : rel=klama, ev=e1, args=[a1, s1, z1, z2, z3], mode=asserted
REF z1 : kind=const, flavor=zo'e, sort=Obj
REF z2 : kind=const, flavor=zo'e, sort=Obj
REF z3 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : p1
```
An occurrence count of zero over the whole interval before `now` says it never
happened in the past, and (crucially) **nothing about the future**. The
recurrence is a property of the one event over its interval, not a collection
of individuated sub-events.

**The remaining tense cmavo populate typed fields on the eventuality:** `ZI`
sets `time.distance`; `ZEhA` sets `timeInterval.extent`; `FAhA`+`VA` set
`space.direction` and `space.distance`; `VEhA`/`VIhA`/`MOhI` set typed spatial
extent, dimensionality, and motion; `TAhE` (`ta'e klama`) adds a recurrence
distribution (`ru'inai` is intermittent); `ROI` (`ci roi`) adds an
`occurrenceCount` recurrence with a quantity object; `ZAhO` (`ba'o klama`)
sets `aspect`; and `CAhA` (`ka'e klama`) sets `actuality`. Bare generated
events have no default actuality. **`KI`** makes the typed tense/adjunct value a
sticky default for later eventualities. **`cu'e`** produces a tense question
with a `tenseQuestion` parameter. **Tense as sumtcita** (`ca lo nu broda`)
anchors the matrix event's time to the abstraction event. **Tense negation**
(`na'e`/`naku` on the tense) applies scalar or contradictory structure to the
typed temporal value.

## Chapter 11 — direct abstraction outputs (`RFY` shorthand)

**11.13 `mi nelci le nu mi limna`** — "I like (my) swimming" (`nu` event abstraction as a term).
```
⟨frame u1/e0/a1/a2⟩
EV  en : denotation=referential, sort=eventuality, content=fr        -- nu output; no actuality forced
PRD pn : rel=limna, ev=en, args=[a1, z1, z2], mode=asserted
REF z1 : kind=const, flavor=zo'e, sort=Obj
REF z2 : kind=const, flavor=zo'e, sort=Obj
FRM fr : pn
EV  e1 : tense=?, caha=?
PRD p1 : rel=nelci, ev=e1, args=[a1, en, z3], mode=asserted
REF z3 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : p1
```
Aktionsart variants change the direct output's sort: `mu'e` →
`eventuality/achievement`, `pu'u` → `eventuality/process`, `zu'o` →
`eventuality/activity`, and `za'i` → `eventuality/state`.

**11.28 `… le ka mi prami ce'u`** — property (one `ce'u`); **two `ce'u`** = a relation (distinct `PAR`s).
```
PAR c1 : sort=entity, role=propertySlot
EV  ek : tense=?, caha=?
PRD pk : rel=prami, ev=ek, args=[a1, c1], mode=asserted
FRM fk : pk
REF rk : category=constant, sort=relation, body=fk, parameters=[c1], arity=1
```
`le ka ce'u prami ce'u` gives the relation-sort referent two distinct
`propertySlot` parameters. **`ni` (11.33)** `le ni le pixra cu blanu` emits a
referent of sort `amount` with the abstraction body and optional scale.
**`jei`** emits a
truth-value referent. **`si'o`/`li'i`/`su'u`** emit concept, experience, or
abstract-nature referents respectively, with `mind` where required.

**11.42 `mi djuno le du'u la .frank. cu bebna`** — proposition (`du'u`); factivity belongs to `djuno`, not `du'u`.
```
⟨frame u1/e0/a1/a2⟩
REF fr : kind=const, flavor=la, sort=Obj ; +cmene-clause(fr, "frank")
EV  eb : tense=?, caha=?
PRD pb : rel=bebna, ev=eb, args=[fr, z1], mode=asserted
REF z1 : kind=const, flavor=zo'e, sort=Obj
FRM fd : pb
RFY rd : kind=du'u, body=fd, abstracted=[]                          -- abstracts nothing
REF D1 : kind=const, flavor=le, sort=Obj ; +LE-clause(D1, ⟨proposition rd⟩)
EV  e1 : tense=?, caha=?
PRD p1 : rel=djuno, ev=e1, args=[a1, D1, z2, z3], mode=asserted      -- I know D1 (subject z2, epistemology z3)
REF z2 : kind=const, flavor=zo'e, sort=Obj
REF z3 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : p1
```
That Frank is a fool follows from `djuno` (factive), **not** from `rd`. **`se du'u`** = `du'u`'s x2 = the **sentence (a `SGN`) expressing `rd`** — used to fill linguistic-behaviour places (`cusku le se du'u …` = "says *that* …", distinct from quoting the exact words `lu … li'u`).

**11.49 `mi djuno le du'u ma kau pu klama le zarci`** — indirect question (`kau` focus).
```
PAR q1 : sort=entity, role=argumentQuestion
EV  ek : tense=pu, caha=?
PRD pk : rel=klama, ev=ek, args=[q1, s1, z1, z2, z3], mode=asserted
FRM fd : pk
QUESTION q2 : kind=argument, mode=indirect, body=fd, slots=[{parameter:q1, role:answer}], focus=q1
RFY rd : kind=du'u, body=fd, abstracted=[], embeddedQuestions=[q2]
… (le du'u rd → D1; djuno(e1; a1, D1, z, z); frame; s1 = le zarci)
```
**`tu'a` (11.64)** `mi troci tu'a le vorme` → `RFY rt : kind=su'u, body=⟨R[do-with](et; z_agent, V)⟩, abstracted=[et]`, `V = le vorme`; `troci(e1; a1, ⟨le su'u rt⟩, z)`. **`jai`** is the converse, raising the abstraction's argument into x1 at the selbri level (`le jai rinka be le nu do morsi` = "the one who caused your death").

## Chapter 12 — lujvo (atomic relations; mechanical metadata only for nonce lujvo)

**12.36 `la .ma([s)] cu dalmikce le gerku`** — "Mary is a vet for the dog" (`dalmikce` = `danlu mikce`, "animal doctor"; asymmetrical place-merge).
```
⟨frame u1/e0/a1/a2⟩
REF M : kind=const, flavor=la, sort=Obj ; +cmene-clause(M, "maris")
REF g1 : kind=const, flavor=le, sort=Obj ; +LE-clause(g1, gerku)
EV  e1 : tense=?, caha=?
PRD p1 : rel=dalmikce, ev=e1, args=[M, g1, z1, z2, z3], mode=asserted   -- ATOMIC rel; args are the merged places
REF z1 : kind=const, flavor=zo'e, sort=Obj                              -- species (d2)
REF z2 : kind=const, flavor=zo'e, sort=Obj                              -- ailment (m3)
REF z3 : kind=const, flavor=zo'e, sort=Obj                              -- treatment (m4)
FRM f1 : p1
```
The dictionary definition is authoritative for `dalmikce`'s meaning and five
places. The graph therefore uses the dictionary lujvo as an atomic relation and
emits **no `REL` object** for it. A spelling-derived explanation alongside the
definition would invite consumers to treat an advisory lujvo decomposition as
normative semantics. The same rule applies to dictionary `ctigau` (`citka gasnu`,
"feeder") and `gerzda` (`gerku zdani`, "kennel"): neither receives a
mechanical decomposition object, and the graph does not infer an implicit
`nu`-abstraction or participant mapping from the component words.

**Nonce lujvo — `ti mlatyzda`** (`mlatu zdani`, with `mlatyzda` absent from the
dictionary). When morphology supplies a complete rafsi decomposition and every
rafsi resolves to its source word, `REL` retains only those mechanical facts:
```
REF t1 : kind=const, flavor=ti, sort=Obj
EV  e1 : tense=?, caha=?
PRD p1 : rel=mlatyzda, ev=e1, args=[t1], mode=asserted
         diagnostics=[relation place structure is unavailable; only places required by explicit assignments are represented]
REL rMZ : relation=mlatyzda, sourceWords=[mlatu,zdani],
          expansion={kind:lujvo, sourceWords:[mlat,zda], rafsiBindings:[]}
FRM f1 : p1
```
There is deliberately no `placeStructure`: a nonce lujvo keeps the same
unknown-place-structure warning and represents only places required by explicit
assignments. `sourceWords` contains resolved full words, while
`expansion.sourceWords` preserves the morphology-derived rafsi; optional
`rafsiBindings` records only genuinely context-sensitive rafsi resolution. None
of this metadata enters a `FRM`. This rafsi rule does not invent an expansion
for a `zei` compound such as `xy. zei kantu`.

## Chapter 13 — attitudinals & evidentials (the `DSP` tier; no truth value, never in `FRM`)

CLL 13.2 is explicit: attitudinals "make no claim… have no truth value, nor do they directly affect the truth value of a bridi that they modify." So a `DSP` never enters `f1` and never changes whether the host is true. The pure-emotion vs propositional-attitude split is then about **where the host bridi sits**, not about its truth value.

**13.x `.ui mi klama le zarci`** — "I'm going to the store — yay!" (**pure emotion**: host stays asserted).
```
⟨frame u1/e0/a1/a2⟩
REF s1 : kind=const, flavor=le, sort=Obj ; +LE-clause(s1, zarci)
EV  e1 : tense=?, caha=?
PRD p1 : rel=klama, ev=e1, args=[a1, s1, z1, z2, z3], mode=asserted
REF z1 : kind=const, flavor=zo'e, sort=Obj
REF z2 : kind=const, flavor=zo'e, sort=Obj
REF z3 : kind=const, flavor=zo'e, sort=Obj
DSP d1 : family=emotion, relation=happiness, experiencer=a1, target=f1, anchor=u1, polarity=positive, assertionEffect=none
FRM f1 : p1                                                          -- the going IS asserted; d1 is additive
```

**13.3 `.au mi klama le zarci`** — "Would that I were going to the store!" (**propositional attitude**: host relocated to a hypothetical world, *not* asserted of reality).
```
⟨frame u1/e0/a1/a2⟩
REF s1 : kind=const, flavor=le, sort=Obj ; +LE-clause(s1, zarci)
EV  e1 : tense=?, caha=?
PRD p1 : rel=klama, ev=e1, args=[a1, s1, z1, z2, z3], mode=asserted    -- (within the reified proposition only)
REF z1 : kind=const, flavor=zo'e, sort=Obj
REF z2 : kind=const, flavor=zo'e, sort=Obj
REF z3 : kind=const, flavor=zo'e, sort=Obj
FRM fp : p1
RFY rp : kind=du'u, body=fp, abstracted=[]                            -- the proposition the attitude is about
DSP d1 : family=propositionalAttitude, relation=desire, experiencer=a1, target=fp, anchor=u1, assertionEffect=hostSubordinated
FRM f1 : ⟨empty⟩                                                      -- NO at-issue real-world claim
```
The going is **not** in any asserted real-world `f1` — `rp` characterizes the hypothetical world `d1` reacts to (CLL: "an internal hypothetical world… distinct from the world as it really is"; from "I hope George wins" you may conclude nothing about George). The DSP did not make the going false; it relocated it. (`.ei` obligation, `.ai` intent, `.e'a` permission, `.e'u` suggestion behave the same.) To actually assert the going you must split it into its own sentence — the diagnostic that propositional attitudes do not assert their host.

**Evidentials set the host's force (gismu-linked):**
- **`za'a` (`zgana`, "I observe")** — host **asserted**, source-marked: `DSP d : family=evidential, relation=zgana, experiencer=a1, target=⟨the proposition⟩` **and** the bridi stays in `f1` (asserted, indisputable-from-observation).
- **`ti'e` (`tirna`, hearsay) / `ru'a` (`sruma`, assumption)** — host **not** speaker-asserted: bridi reified into the gismu's `du'u` (sits as the evidential's target), `f1` empty.
- **`ca'e` (define/performative)** — host **made true by the utterance**: `PRD … mode=performative` in `f1` (the `ca'e`/definition case).

**Modifiers (no truth effect):** `dai` → `experiencer` shifts to a
non-speaker / empathic referent; `pei` introduces an `attitudeQuestion`
parameter and an attitude-kind question over the displayed-content field;
`ge'e` → explicit null DSP; `bu'o`/`bu'oi`/`bu'onai` →
`phase=start|continue|end` (the displayed-tier `ZAhO`, distinct from an
asserted `ba'o prami` claim); `CAI` (`cai`/`sai`/`ru'e`/`cu'i`) →
`intensity`/`polarity`. **Insincerity** (feeling `.ui` you don't feel) =
**infelicity, not falsity** — there is no truth value to negate. **Scope by
placement:** sentence-initial/post-selbri ⇒ `target=e1`; after a sumti ⇒
`target` = that sumti.

## Chapter 14 — connectives (the `FRM` layer; the locus fixes sharing)

**14.49/14.50 `mi klama le zarci gi'e nelci la .djan.`** — compound bridi sharing x1.
```
⟨frame u1/e0/a1/a2⟩
REF Z : kind=const, flavor=le, sort=Obj ; +LE-clause(Z, zarci)
REF J : kind=const, flavor=la, sort=Obj ; +cmene-clause(J, "djan")
EV  eA : tense=?, caha=?
PRD pA : rel=klama, ev=eA, args=[a1, Z, z1, z2, z3], mode=asserted     -- a1 shared (the one explicit x1)
EV  eB : tense=?, caha=?
PRD pB : rel=nelci, ev=eB, args=[a1, J, z4], mode=asserted
REF z1 : kind=const, flavor=zo'e, sort=Obj
REF z2 : kind=const, flavor=zo'e, sort=Obj
REF z3 : kind=const, flavor=zo'e, sort=Obj
REF z4 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : and(pA, pB)
```

**14.58/14.59 — the central non-sharing result.** `klama la .nu,iork. la .finyks. gi'e klama la .nu,iork. la .rom.` ("a goer to NY from Phoenix, and to NY from Rome"). x1 is **omitted in both** ⇒ the two x1's are **distinct `zo'e`**, *not* forced equal — and crucially the omitted x4/x5 (route/means) are also each their own `zo'e`, so the two routes can differ (if x1 were shared, "nothing special about x1" would force the routes equal too — absurd).
```
⟨frame u1/e0/a1/a2⟩
REF NY : kind=const, flavor=la, sort=Obj ; +cmene-clause(NY, "nuiork")
REF Ph : kind=const, flavor=la, sort=Obj ; +cmene-clause(Ph, "finyks")
REF Ro : kind=const, flavor=la, sort=Obj ; +cmene-clause(Ro, "rom")
EV  eA : tense=?, caha=?
PRD pA : rel=klama, ev=eA, args=[zA, NY, Ph, z1, z2], mode=asserted    -- own x1 zA, own route z1, means z2
EV  eB : tense=?, caha=?
PRD pB : rel=klama, ev=eB, args=[zB, NY, Ro, z3, z4], mode=asserted    -- DISTINCT x1 zB, route z3, means z4
REF zA : kind=const, flavor=zo'e, sort=Obj
REF zB : kind=const, flavor=zo'e, sort=Obj
REF z1 : kind=const, flavor=zo'e, sort=Obj
REF z2 : kind=const, flavor=zo'e, sort=Obj
REF z3 : kind=const, flavor=zo'e, sort=Obj
REF z4 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : and(pA, pB)
```
Writing `da` in x1 (14.59) replaces `zA`/`zB` with **one** bound `var` (`EX(X, and(pA[zA:=X], pB[zB:=X]))`) — forcing x1 identity while leaving the routes distinct. This is the whole "elided place = its own `zo'e`, never silently shared" discipline. Logical sumti connection distributes one surface predication, but it does not make omitted non-connected places corefer: overt non-connected terms stay shared by id, while omitted places get branch-local `zo'e` referents.

**14.x sumti connection `do .e mi`** — one shared predication, distributing.
```
PRD p1 : rel=R…, ev=e1, args=[a2, overt-shared-args, branch zo'e…], mode=asserted -- the `do` instance
PRD p2 : rel=R…, ev=e2, args=[a1, overt-shared-args, branch zo'e…], mode=asserted -- the `mi` instance
FRM f1 : and(p1, p2)
```
The two predications share overt non-connected arguments, but omitted places are separate branch-local `zo'e` referents. This keeps CLL 14.26 equivalent to the corresponding `.ije` expansion with respect to unspecified origins, routes, and means.

**Realization algebra (one `FRM` node per connection):** `.a`→`or`, `.e`→`and`, `.o`→`iff`, `.u`→ first asserted + second `mode=inert`; `na`/`nai`/`se` transform structurally — `na.a`→`or(not p,q)` (truth-functionally implication), afterthought `.enai`→`and(p, not q)`, forethought `ganai ... gi`→`or(not p, q)`, forethought `ge ... ginai`→`and(p, not q)`, `se.u`→`inert(p), assert(q)`. **Connective question `ji`** (`do ji mi`) introduces a `connectiveQuestion` parameter of sort `connective`; the question has `kind=connective` and a typed answer slot. **Forethought `ge…gi`** = same `FRM` nodes, prefix order. **Termset `nu'i…nu'u` / `pe'e`** = parallel connection of several places: equal-length termsets zip corresponding terms, while unequal branches replay surrounding terms per branch, so in CLL 14.74 the same following `le briju` is x2 in the `mi` branch and x3 in the `do ce'e le zarci` branch. **Non-logical `joi`/`ce`/`ce'o` (14.15)** do **not** enter `FRM`: `joi`→a `gunma` composite, `ce`→a `selcmi` set, `ce'o`→an ordered sequence — all **composite referents** fed to one predication. **`fa'u`** first introduces a respectively-paired composite, but when the correspondence itself is truth-conditional it is promoted to `respectivelyDistribution`: CLL 14.124 zips James/George against two distinct sister witnesses, and CLL 14.133 zips John/Frank against two tagged/adjunct branch formulas. For non-logical termsets with tagged components (CLL 14.131/14.132), branch-local adjuncts carry `component` so each adjunct is tied to the relevant member of the composite argument; `fa'u` 14.133 additionally exposes the branch correspondence as parallel streams.

## Chapters 15–16 — negation & quantifier scope (reference-by-id)

**15.2 `mi na klama le zarci`** — "It is not the case that I go to the store" (contradictory bridi negation).
```
⟨frame u1/e0/a1/a2⟩
REF s1 : kind=const, flavor=le, sort=Obj ; +LE-clause(s1, zarci)
EV  e1 : tense=?, caha=?
PRD p1 : rel=klama, ev=e1, args=[a1, s1, z1, z2, z3], mode=asserted
REF z1 : kind=const, flavor=zo'e, sort=Obj
REF z2 : kind=const, flavor=zo'e, sort=Obj
REF z3 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : not(p1)
```
**`na'e` (scalar)** `mi na'e klama` keeps `relation=klama` and adds
`scalarNegation={kind:otherThan, introducedBy:na'e, scale:..., argumentScope:[x1]}`
to the predication: it is a positive claim at another point on a scale, not
contradictory negation. `be ci'u ...` supplies the scale definition. **`na'i`
(metalinguistic)** → `DSP d : family=metalinguistic, relation=na'i,
target=⟨the bridi/term⟩, targetFocus=clause|predicate,
assertionEffect=metalinguisticallyVoided` (flags a false presupposition; no
truth value, not a `not`).

**16.x `naku` boundary inversion — `naku ro da poi gerku cu blabi`** ≡ `su'o da poi gerku cu na blabi` ("not all dogs are white" = "some dog is not white").
```
⟨frame u1/e0/a1/a2⟩
REF v : kind=var, sort=Obj
PRD pg : rel=gerku, ev=eg, args=[v, z1], mode=(restriction)
PRD pb : rel=blabi, ev=eb, args=[v, z2], mode=asserted
REF z1 : kind=const, flavor=zo'e, sort=Obj
REF z2 : kind=const, flavor=zo'e, sort=Obj
FRM fU : and(ALL(v, imp(pg, pb)), EX(v, pg))          -- ro da poi gerku cu blabi (with domain import)
FRM f1 : not(fU)
-- equivalently (naku crossing the quantifier inverts ∀→∃ and pushes the negation in):
FRM f1' : EX(v, and(pg, not(pb)))                      -- su'o da poi gerku cu na blabi
```
The two forms `f1`/`f1'` are the recorded **surface readings**; `naku` crossing a quantifier flips `∀↔∃` and crossing a connective forces DeMorgan — truth-preserving equivalences the model notes but does not collapse.

**16.48/16.49 — universal "any" (no import) vs restricted universal (import).** `ro da poi klama le zarci cu cadzu le foldi` carries import (goers exist); the conditional `ro da zo'u da go klama le zarci gi cadzu le foldi` does not.
```
-- restricted universal (16.48): goers exist
FRM fA : and(ALL(v, imp(klama(v, Z,…), cadzu(v, F,…))), EX(v, klama(v, Z,…)))
-- import-free "any" (16.49): NO claim that goers exist
FRM fB : ALL(v, iff(klama(v, Z,…), cadzu(v, F,…)))
```
**16.50/16.55 — existential "any"** (`mi nitcu da poi tanxe gi'e bramau ti`, "I need any box bigger than this"): bind the variable in a **subordinate** bridi's prenex so its existence rides only on the (possibly non-occurring) need-event.
```
EV  en : tense=?, caha=?
PRD pp : rel=ponse, ev=en, args=[a1, X], mode=asserted
PRD pt : rel=tanxe, ev=et, args=[X, z1], mode=asserted
PRD pm : rel=bramau, ev=em, args=[X, ti, z2], mode=asserted
FRM fb : EX(X, and(pp, pt, pm))                        -- the variable scoped INSIDE the event
RFY rn : kind=nu, body=fb, abstracted=[en]
PRD p1 : rel=nitcu, ev=e1, args=[a1, ⟨lo nu rn⟩, z3], mode=asserted
FRM f1 : p1                                            -- no real-world box asserted
```
**`no da` (16.x)** `no da broda` → `NO(v, broda(v))` = `not(EX(v, broda(v)))`. **`da`/`de`/`di` ordering** in one prenex = nested quantifier nodes in surface order (`EX(X, ALL(Y, …))` ≠ `ALL(Y, EX(X, …))` — the ∃∀/∀∃ distinction the FRM nodes exist to preserve). **Re-quantified `da`** (`ci da poi prenu ... pa da`) binds a fresh selected variable for the second quantifier, records the earlier variable as its witness-set source, and copies the source restriction (`prenu`) onto the selected variable. **Grouping termsets** (16.7: `ci gerku ce'e re nanmu cu batci`; equivalently `nu'i ci gerku re nanmu nu'u cu batci`) are not either nesting order; they emit one coequal `quantifierBundle` with bindings for the dog and man variables and a shared `batci` body, so the same fixed three dogs and same fixed two men participate in the cross-product reading.

## Chapters 17–18 — letterals (sign referents) & mekso (MEX)

**17.x letteral as pro-sumti — `lo gerku ... gy. ...`** ("the dog … it[g] …"). Once anaphora is resolved, the lerfu points at the same existing `REF` id; there is no public handle field.
```
⟨frame u1/e0/a1/a2⟩
REF g1 : kind=const, flavor=lo, sort=Obj                            -- gy. (initial of gerku) resolves back to this id
… (g1 then appears by shared id wherever `gy.` recurs) …
SGN ly : kind=letteral, text="g", letterals=[{kind:glyph, sourceWords:[gy], value:g}]
```
The three **uses** are: **character** (`me'o gy.` = the letter as an object → a sign-sort referent); **pro-sumti** (above — the already existing referent by shared id; a lerfu string is **one** referent); and **mekso variable** (a `MEX` literal `{kind:variable, value:"gy"}`). A letteral sign's `letterals` array distinguishes `glyph`, `digit`, `shift`, `characterCode`, and `compound` units and retains their source words. For example, `denpa bu` is one glyph unit with `sourceWords=[denpa,bu]` and `buDepth=1`; current output does not add an evocative relation object.

**18.x value vs sign — `li vo su'i re du li xa`** ("4 + 2 = 6", values) and `me'o vo su'i re` (the unevaluated expression).
```
⟨frame u1/e0/a1/a2⟩
MEX m1 : operator=add, operands=[4, 2]                               -- the mekso tree 4 + 2
QTY L1 : form=exact, value={mathExpression:m1}, scale=count          -- li (vo su'i re)
QTY L2 : form=exact, value={integer:6}, scale=count                  -- li xa
EV  e1 : tense=?, caha=?
PRD p1 : rel=du, ev=e1, args=[L1, L2], mode=asserted                 -- the two values are identical
FRM f1 : p1
```
`me'o vo su'i re` instead yields `SGN q : kind=mathExpression,
text="vo su'i re", denotes=m1` (the **unevaluated** expression as a sign —
`me'o` ≠ `li`, so `me'o vo su'i re` ≠ `li xa` even though the value would be
6). **MOI relations** (number→relation): `mei` (`lo ci mei` = a mass/set of 3,
`mei(x, 3)`), `moi` (`la .alis. cu ralju lo'i prenu vo moi` →
`moi(alis, set, 4, rule)` = 4th), `si'e` (portion), `cu'o` (probability).
**`nu'a` (18.x)** `nu'a su'i` converts the operator `su'i` back to a
**relation** (a selbri: x1 is the sum of x2, x3, …). Internally operators are
VUhU (`su'i`=+, `vu'u`=−, `pi'i`=×, `te'a`=^); operands are numbers,
`li`-quantities, lerfu-variable literals, special numbers (`pi`=π,
`te'o`=e, `ka'o`=i), and `xi`-subscripts. Infix/forethought/RPN are surface
orderings of the one `MEX` tree. **Quantifier use** (`re lo broda`, a number as
a sumti-prefix) feeds `CARD`/`PA` into a quantifier `FRM`-node (ch. 6/16), the
fourth and last door from `MEX` into the core. Logical connectives at that door
lift to formula structure: `vei ci .a vo` as a quantifier is an `or` over two
`CARD` scopes, and connected operators such as `su'i je pi'i` in
`li ... du ...` become an `and` over two full identity formulas (`add` vs
`multiply`), not a fused `MEX` operator.

## Chapter 19 — text structure (SEQ, quotation, parentheticals)

**19.1 `mi klama le zarci .i do cadzu le bisli`** — two sentences, vague relationship (a `SEQ`, **not** a conjunction).
```
⟨frame u1/e0/a1/a2⟩
REF Z : kind=const, flavor=le, sort=Obj ; +LE-clause(Z, zarci)
REF B : kind=const, flavor=le, sort=Obj ; +LE-clause(B, bisli)
PRD pA : rel=klama, ev=eA, args=[a1, Z, z1, z2, z3], mode=asserted
PRD pB : rel=cadzu, ev=eB, args=[a2, B, z4], mode=asserted
REF z1 : kind=const, flavor=zo'e, sort=Obj
REF z2 : kind=const, flavor=zo'e, sort=Obj
REF z3 : kind=const, flavor=zo'e, sort=Obj
REF z4 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : pA
FRM fb : pB
SEQ s  : items=[u1, ub], relation=same-topic-continuation          -- TRUTH-VALUELESS sequence relation
```
`.i` is **never** a truth function; the two bridi each keep their own truth
value and import. The unit relation `same-topic-continuation` records the
unconnected sentence boundary. **`.ije`** gives the sequence a connected
`content` formula such as `and(f1, fb)`. **`.i bo`** / **`tu'e…tu'u`** group
sequence items more tightly. **NIhO** uses the tagged `paragraph-boundary`
relation with `new-topic` or `resume-prior-topic` transitions.

**19.4 structured vs opaque quotation.** `la .djan. cusku lu mi klama li'u` — `lu…li'u` is a **nested `UTT`** with reachable referents.
```
⟨frame u1/e0/a1/a2⟩
REF J : kind=const, flavor=la, sort=Obj ; +cmene-clause(J, "djan")
UTT u2 : force=assert, content=f2, ev=e2, speaker=J, audience=z1      -- quoted by the sign; its OWN roles
PRD p2 : rel=klama, ev=ek, args=[J2, Zq, …], mode=asserted           -- inner `mi` = u2.speaker (= J, by content)
REF J2 : kind=const, indexical=⟨u2.speaker⟩, sort=Obj
REF Zq : kind=const, flavor=le, sort=Obj ; +LE-clause(Zq, zarci)
FRM f2 : p2
REF z1 : kind=const, flavor=zo'e, sort=Obj
EV  e1 : tense=?, caha=?
PRD p1 : rel=cusku, ev=e1, args=[J, u2, z2, z3], mode=asserted        -- John expresses the utterance u2
REF z2 : kind=const, flavor=zo'e, sort=Obj
REF z3 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : p1
```
Inner `mi`→`u2.speaker` (C-17); referents inside `u2` are reachable from outside by shared id (the 7.48 cross-quotation case). **Opaque** `lo'u mi klama le'u` / `zo klama` / `zoi gy. ... gy.` → a sealed `SGN kind=quotation` (token sequence, **no** internal `UTT`, no reachable referents). **`la'o .X. ... .X.`** = a name from foreign text (`la me zoi`). **`la'e`/`lu'e`** = the reference↔referent shifts (the `zo .bab.` word / `la .bab.` named thing / `la'e zo .bab.` referent triad).

**19.x topic-comment `lo cukta zo'u mi pinxe`** — vague link (genuinely underspecified, exposed not fabricated).
```
⟨frame u1/e0/a1/a2⟩
REF C : kind=const, flavor=lo, sort=Obj ; +PRD cukta(C,…) restrictive  -- the topic
PRD p1 : rel=pinxe, ev=e1, args=[a1, z1], mode=asserted
PRD pr : rel=R[topic-of], ev=er, args=[C, e1], mode=asserted          -- vague: C's role in the comment is UNDERSPECIFIED
REF z1 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : and(p1, pr)
```
`zo'u` here is the same construct as the prenex (16.8); the pre-`zo'u` string can carry both quantifier bindings and a topic, and the topic's argument place in the comment is left as the open `R[topic-of]`.

**19.x parentheticals — `mi klama le zarci .i to mi nelci la .djan. toi`** (`to…toi` aside).
```
⟨frame u1/e0/a1/a2⟩  -- host
PRD p1 : rel=klama, ev=e1, args=[a1, Z, …], mode=asserted ; FRM f1 : p1
UTT u3 : force=parenthetical, content=f3, ev=e3, speaker=a1, audience=a2   -- shares host deictic ground
PRD p3 : rel=nelci, ev=e3b, args=[a1, J, z9], mode=asserted ; FRM f3 : p3
UTT u1 : … , asides=[u3]                                              -- UNORDERED aside edge, outside FRM
```
`u3` hangs off an unordered `aside` edge (not in `f1`); if the bracket held several **unconnected** sentences its content would be a `SEQ` (truth-valueless); if they were `.ije`-connected, that connection is an ordinary `FRM` inside one `SEQ` item (grouping = precedence). `sei…se'u` is the same with `force=parenthetical` mid-bridi. A non-logical expression spliced where a connective operand is expected **does not typecheck** — represented without imparting meaning. **`MAI`** (`pamai`) = an ordinal **label on a `SEQ` item**; **`XI`** (`xi re`) = a `subscript` yielding a **distinct** global symbol; **`BAhE`/`FUhE…FUhO`** = a `DSP` emphasis/attitude-scope annotation. **`SI`/`SA`/`SU`/`FAhO`** = token-stream erasure/end-of-text, applied **pre-semantically** (like rafsi decomposition and the magic-word rules) and therefore **absent** from the final structure.

---

## Gotchas found & doc revisions during this pass

See the closing summary in the accompanying message. In brief: (1) **tanru** use one uniform schema — tertau asserted + seltau as a reified kind `⟨ka ce'u S⟩` linked by the vague `R`, asserting neither `S(x)` nor a concrete seltau referent (revised under C-26 from the earlier two-sub-case form, which over-committed); (2) **asserted adjunct predications** conjoined into content (fall under `na`); (3) **restricted universals** carry categorical domain-nonemptiness import (CLL 16.8), distinct from the import-free "any". A later cross-check against jbotci `tersmu`'s Lean prelude (C-26) corroborated the model broadly and prompted the uniform tanru schema plus the `Proposition`/`TruthValue` sorts and the `gi'e`/`.e` sharing fix; no other definitions required revision; all remaining chapters validated against CLL as-is.
