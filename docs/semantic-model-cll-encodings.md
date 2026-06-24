# Lojban → Discourse Object Model: Worked Encodings (CLL 5–19)

*Companion to `lojban-discourse-object-model.md`. Every example is encoded in the canonical **reference-by-id** form (C-22): one object per line, compound formulas referencing sub-formulas by id, nothing nested literally. CLL is authoritative except for the gadri (xorlo/guskant). Re-verified against CLL and the community dictionary section by section; gotchas found during this pass are collected at the end.*

**Reading the notation.** `KIND id : attr=val, …` per the model doc. The standing frame
```
UTT u1 : force=assert, content=f1, ev=e0, speaker=a1, audience=a2
REF a1 : kind=const, sort=Obj
REF a2 : kind=const, sort=Obj
EV  e0 : tense=now, caha=ca'a
```
is present in every example; after the first occurrence per chapter I write it as `⟨frame u1/e0/a1/a2⟩`. A bare `EV en` carries `tense=?, caha=ca'a` (asserted main bridi) unless marked. Elided places are explicit `zo'e` `REF`s.

---
## Chapter 5 — selbri

**5.7 `la .djan. barda nanla`** — "John is a big boy" (intersective tanru: x is both big and a boy).
```
UTT u1 : force=assert, content=f1, ev=e0, speaker=a1, audience=a2
REF a1 : kind=const, sort=Obj
REF a2 : kind=const, sort=Obj
EV  e0 : tense=now, caha=ca'a
REF d1 : kind=const, flavor=la, sort=Obj
SGN w1 : kind=word, text="djan"
EV  e2 : tense=?, caha=ca'a
PRD pN : rel=cmene, ev=e2, args=[w1, d1, z1], mode=incidental      -- w1 names d1 (namer z1)
REF z1 : kind=const, flavor=zo'e, sort=Obj
EV  eT : tense=?, caha=ca'a
PRD pT : rel=nanla, ev=eT, args=[d1, z2, z3], mode=asserted         -- tertau (primary): d1 is a boy; supplies the one event
EV  eK : tense=?, caha=ca'a
RFY k1 : kind=ka, body=⟨barda(eK; ce'u, z4, z5)⟩, abstracted=[ce'u]  -- seltau reified as the kind "being big"
PRD pR : rel=tanru, tanruLink={head:pT, modifier:k1, label:barda-nanla}, args=[d1, k1], mode=asserted   -- vague tanru link: d1 stands to the "big" kind
REF z2 : kind=const, flavor=zo'e, sort=Obj
REF z3 : kind=const, flavor=zo'e, sort=Obj
REF z4 : kind=const, flavor=zo'e, sort=Obj
REF z5 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : and(pT, pR)
```
`la djan` claims **this one** referent `d1` is a boy etc. — not everyone so named — and `pN` carries that `d1` is so-named (existential import via the asserted locution). Per 0.F's uniform schema the seltau `barda` is the reified kind `k1` linked by the vague `R`; the **intersective** reading resolves `R` to instantiation (unfolding to `barda(d1)`), so the structure records the link rather than asserting a separate `barda(d1)` conjunct.

**5.13 `ta cinfo kerfa`** — "that is a lion-mane" (**asymmetrical** tanru: the seltau is *not* predicated of x).
```
⟨frame u1/e0/a1/a2⟩
REF t1 : kind=const, indexical=⟨demonstratum of u1⟩, sort=Obj      -- ta: the thing pointed at = the mane
EV  eT : tense=?, caha=ca'a
PRD pT : rel=kerfa, ev=eT, args=[t1, z1, z2], mode=asserted         -- tertau (primary): t1 is a mane (of body z1); the one event
EV  eK : tense=?, caha=ca'a
RFY k1 : kind=ka, body=⟨cinfo(eK; ce'u, z3)⟩, abstracted=[ce'u]      -- seltau reified as the kind "being a lion"
PRD pR : rel=tanru, tanruLink={head:pT, modifier:k1, label:cinfo-kerfa}, args=[t1, k1], mode=asserted       -- vague link: t1 stands to the "lion" kind
REF z1 : kind=const, flavor=zo'e, sort=Obj
REF z2 : kind=const, flavor=zo'e, sort=Obj
REF z3 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : and(pT, pR)
```
Under 0.F's uniform schema **no lion referent is introduced at all**: the seltau `cinfo` is the reified kind `k1`, and the typed `tanru` link connects the mane `t1` to that kind. Nothing asserts a lion exists — the selbri alone doesn't entail one — so neither the intersective `cinfo(t1)` (the mane is not a lion) nor a concrete `cinfo(y1)` is claimed; the lion-relationship lives entirely in the unresolved `R`. (Same shape handles `junla dadysli`, `rokci cinfo`, etc. — the reading is just which `R`.)

**Deltas for the other chapter-5 constructs** (each changes only a little):
- **`je` (5.6-style symmetrical, `remna nakni` "man")** — intersective, **drop `R`**: `f1 : and(pA, pB)` with `remna(eA; x, …)`, `nakni(eB; x, …)` sharing `x`. **`ja`/`jo`/`naja`** → same two `PRD`s under `or`/`iff`/`imp` in `f1`.
- **`joi` (5.58 `blanu joi xunre`)** — a `gunma`-of-properties composite `(blanu ⊕ xunre)` fed to a colour predication (collective), not an `and` in `f1`.
- **`be` (5.64 `ti xamgu be do bei mi`)** — the seltau `PRD` has filled args: `xamgu(eS; ti, do, mi) mode=asserted`, no `zo'e` in x2/x3.
- **`co` (5.79)** — identical bag to the un-inverted tanru; word order only.
- **`SE` (5.110 `do se prami mi`)** — `PRD p1 : rel=se:prami, ev=e1, args=[do, mi], mode=asserted` (≡ asserts `prami(mi, do)`); here `do`→`u1.audience`, `mi`→`u1.speaker`.
- **`me` (5.99 `la .baltazar. cu me le ci nolraitru`)** — `me(e1; B, K) mode=asserted`, `K` the `le ci nolraitru` constant (carrying `mei(K, 3)` and the `LE-clause`).
- **scalar (5.117 `… na'e cadzu klama …`)** — `klama(e1; …) mode=asserted` (he *does* go) + `na'e:cadzu(e2; …) mode=asserted` + `R`; `na'e:` wraps the relation, asserting a different point on the manner-scale.

## Chapter 6 — sumti

**The canonical claim-by-omission — `lo botpi cu xunre`** ("a bottle is red"). `botpi` = x1 is a bottle for contents x2, of material x3, with **lid x4**; so a lid is asserted to exist.
```
⟨frame u1/e0/a1/a2⟩
REF b1 : kind=const, flavor=lo, sort=Obj
EV  eb : tense=?, caha=ca'a
PRD pb : rel=botpi, ev=eb, args=[b1, z1, z2, z3], mode=incidental   -- lo botpi = zo'e noi botpi; z3 = the LID
REF z1 : kind=const, flavor=zo'e, sort=Obj                          -- contents
REF z2 : kind=const, flavor=zo'e, sort=Obj                          -- material
REF z3 : kind=const, flavor=zo'e, sort=Obj                          -- LID: its existence is asserted (import via pb)
EV  e1 : tense=?, caha=ca'a
PRD p1 : rel=xunre, ev=e1, args=[b1, z4], mode=asserted
REF z4 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : p1
```
Each elided place is its own `zo'e` `REF`; `z3` (the lid) carries existential import through `pb`. To deny the lid you would write `zi'o` in x4 (no `REF`, no import).

**6.6 `le zarci cu barda`** — full `le` expansion shown once, then abbreviated.
```
⟨frame u1/e0/a1/a2⟩
REF s1 : kind=const, flavor=le, sort=Obj
RFY k1 : kind=ka, body=fk, abstracted=[c1]                          -- lo ka ce'u zarci
PAR c1 : sort=Obj, role=ce'u
EV  ek : tense=?, caha=?
PRD pk : rel=zarci, ev=ek, args=[c1, z1, z2], mode=asserted          -- (in fk) ce'u is a market
REF z1 : kind=const, flavor=zo'e, sort=Obj
REF z2 : kind=const, flavor=zo'e, sort=Obj
FRM fk : pk
EV  e3 : tense=?, caha=ca'a
PRD p3 : rel=skicu, ev=e3, args=[a1, s1, a2, k1], mode=incidental     -- I(=speaker) describe s1 to you(=audience)
EV  e1 : tense=?, caha=ca'a
PRD p1 : rel=barda, ev=e1, args=[s1, z3, z4], mode=asserted
REF z3 : kind=const, flavor=zo'e, sort=Obj
REF z4 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : p1
```
The only at-issue claim is `barda(s1)`; that `s1` is a market is the speaker's *description* (`p3`, incidental), so `le` can be false-of-its-noun. Henceforth `LE-clause(x, broda)` abbreviates `p3`+`k1`+`fk`. Note `skicu`'s describer/audience are `u1.speaker`/`u1.audience` (C-17).

**6.9 `lo mlatu cu gerku`** — false, by shared referent identity (not by tier).
```
⟨frame u1/e0/a1/a2⟩
REF c1 : kind=const, flavor=lo, sort=Obj
EV  em : tense=?, caha=ca'a
PRD pm : rel=mlatu, ev=em, args=[c1, z1], mode=incidental            -- c1 noi mlatu (really a cat)
REF z1 : kind=const, flavor=zo'e, sort=Obj
EV  e1 : tense=?, caha=ca'a
PRD p1 : rel=gerku, ev=e1, args=[c1, z2], mode=asserted
REF z2 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : p1
```
`p1` is unsatisfiable because **`c1`** must be both cat (`pm`) and dog (`p1`); one referent, no animal qualifies. `le mlatu cu gerku` (6.7) is *not* contradictory: swap `pm` for `LE-clause(c1, mlatu)` — only "I call it a cat" is claimed, so a dog satisfies it.

**6.17 `lei prenu cu bevri le pipno`** — the mass carries the piano (collective; contradictory skin-colours tolerated).
```
⟨frame u1/e0/a1/a2⟩
REF x1 : kind=const, flavor=le, sort=Obj ; +LE-clause(x1, prenu)     -- the specific people (inner constant)
REF m1 : kind=const, sort=Obj                                        -- the MASS
EV  eg : tense=?, caha=ca'a
PRD pg : rel=gunma, ev=eg, args=[m1, x1], mode=incidental             -- lei = lo gunma be le prenu
REF K1 : kind=const, flavor=le, sort=Obj ; +LE-clause(K1, pipno)
EV  e1 : tense=?, caha=ca'a
PRD p1 : rel=bevri, ev=e1, args=[m1, K1, z1, z2, z3], mode=asserted   -- the MASS carries K1: collective
REF z1 : kind=const, flavor=zo'e, sort=Obj
REF z2 : kind=const, flavor=zo'e, sort=Obj
REF z3 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : p1
```
The carrier is `m1` (a `gunma`), so `bevri` does **not** distribute to each person — exactly CLL's "carried it jointly". **6.18 `loi cinfo cu xabju le fi'ortu'a`** is identical with `flavor=lo` on the inner constant (`lo gunma be lo cinfo`); the "part of the mass" / zoo-lions escape falls out because `m1` is a `lo`-constant (*some* lion-mass), needing no separate quantifier.

**6.24 `lo'i ratcu cu barda`** — the *set* is large.
```
⟨frame u1/e0/a1/a2⟩
REF x1 : kind=const, flavor=lo, sort=Obj                             -- lo ratcu (inner constant)
EV  er : tense=?, caha=ca'a
PRD pr : rel=ratcu, ev=er, args=[x1, z1], mode=incidental
REF z1 : kind=const, flavor=zo'e, sort=Obj
REF S1 : kind=const, sort=Obj                                        -- the SET
EV  es : tense=?, caha=ca'a
PRD ps : rel=selcmi, ev=es, args=[S1, x1], mode=incidental            -- lo'i = lo selcmi be lo ratcu (strict)
EV  e1 : tense=?, caha=ca'a
PRD p1 : rel=barda, ev=e1, args=[S1, z2, z3], mode=asserted           -- the SET is large
REF z2 : kind=const, flavor=zo'e, sort=Obj
REF z3 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : p1
```
`barda` predicates of the set object `S1`; `bunre(S1)` would be a sortal mismatch the model exposes rather than forbids. **6.26 `lo'e cinfo cu xabju le fi'ortu'a`** (typical lion): `REF T:const, flavor=lo'e, descriptor.veridical=false` + a non-veridical `cinfo(T)` descriptor body + `xabju(T, …)`; `T` is the intensional generic.

**6.31 `re do cadzu le bisli`** — "two of you walk on the ice" (outer quantifier ⇒ restricted distributive variable; reference-by-id).
```
⟨frame u1/e0/a1/a2⟩
REF v1 : kind=var, sort=Obj                                          -- re da
EV  em : tense=?, caha=ca'a
PRD pm : rel=me, ev=em, args=[v1, a2], mode=restrictive               -- v1 poi me do  (do = u1.audience)
REF I1 : kind=const, flavor=le, sort=Obj ; +LE-clause(I1, bisli)
EV  e1 : tense=?, caha=ca'a
PRD p1 : rel=cadzu, ev=e1, args=[v1, I1, z1], mode=asserted
REF z1 : kind=const, flavor=zo'e, sort=Obj
FRM fr : and(pm, p1)
FRM f1 : CARD(v1, 2, fr)                                              -- exactly two v among the audience: each walks
```
`re do` = `CARD(v, 2, and(me(v, audience), …))`, distributive. **6.39 `re le ci gerku cu blabi`** (inner + outer): add `REF G : const, flavor=le` + `PRD mei(G, 3) incidental` (the three dogs), restrict `me(v, G)`, and `f1 : CARD(v, 2, and(me(v,G), blabi(v)))`. **6.44 `ci gerku cu blabi`** (indefinite): `f1 : CARD(v, 3, and(gerku(v,…), blabi(v,…)))` — the variable ranges over `gerku` directly, no inner constant.

**6.53 / 6.54 LAhE — `mi viska la'e lu le xunre cmaxirma li'u`** and `mi pu cusku lu'e le vi cukta`.
```
⟨frame u1/e0/a1/a2⟩
SGN Q1 : kind=grammatical, text="le xunre cmaxirma", utt=u2          -- the quoted title (a structured sign)
UTT u2 : force=mentioned, content=fq, ev=eq, speaker=b1, audience=b2
… (fq = the parsed title's content; its roles b1/b2 its own) …
EV  e1 : tense=?, caha=ca'a
PRD p1 : rel=viska, ev=e1, args=[a1, lae[Q1], z1], mode=asserted      -- la'e Q1 = the book the sign denotes
REF z1 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : p1
```
`lae[Q1]` = `lo se sinxa be Q1` (the book). `lu'e` is the converse: `cusku(e1[tense=pu]; a1, lue[K], z)` with `K = le vi cukta`, `lue[K]` a sign for the book (its title). **6.56 `mi troci tu'a le vorme`**: `troci(e1; a1, g, z)` with `RFY g : kind=su'u, body=⟨R(eR; V, z')⟩, abstracted=[eR]`, `V = le vorme` (a raised `co'e` abstraction). **6.57 `… lu'a ri cmalu`**: `ri` is the **same id** as the last referent (the set `S1`); `lu'a[S1]` = its members. **`na'ebo le gerku` (6.60)**: `viska(e1; a1, ⟨na'e: G⟩, z)` — a scalar-other-than over the constant `G = le gerku`.

## Chapter 7 — pro-sumti and pro-bridi

Chapter 7 is the **payoff of desugaring**: anaphora, assignment, reflexives, and assigned pro-bridi all reduce to **reuse of an existing node's id**. There is no anaphora machinery.

**7.32 `mi prami mi`** — reflexive by repetition; `mi`→`u1.speaker` (C-17), one node twice.
```
UTT u1 : force=assert, content=f1, ev=e0, speaker=a1, audience=a2
REF a1 : kind=const, sort=Obj
REF a2 : kind=const, sort=Obj
EV  e0 : tense=now, caha=ca'a
EV  e1 : tense=?, caha=ca'a
PRD p1 : rel=prami, ev=e1, args=[a1, a1], mode=asserted              -- both places = u1.speaker
FRM f1 : p1
```

**7.33 `la .djan. viska le tricu .i ri se jadni le ri jimca`** — all `ri`s collapse to one id.
```
⟨frame u1/e0/a1/a2⟩
REF j1 : kind=const, flavor=la, sort=Obj ; +cmene-clause(j1, "djan")
REF Tr : kind=const, flavor=le, sort=Obj ; +LE-clause(Tr, tricu)
EV  e1 : tense=?, caha=ca'a
PRD p1 : rel=viska, ev=e1, args=[j1, Tr, z1], mode=asserted
REF z1 : kind=const, flavor=zo'e, sort=Obj
REF Br : kind=const, flavor=le, sort=Obj ; +LE-clause(Br, jimca)    -- le ri jimca: the branches OF Tr
EV  ej : tense=?, caha=ca'a
PRD pj : rel=jimca, ev=ej, args=[Br, Tr, z2], mode=incidental        -- branches of body Tr (ri = Tr)
REF z2 : kind=const, flavor=zo'e, sort=Obj
EV  e2 : tense=?, caha=ca'a
PRD p2 : rel=se:jadni, ev=e2, args=[Tr, Br], mode=asserted           -- Tr (ri) is adorned by Br
FRM f1 : p1
FRM f2 : p2
SEQ s  : items=[⟨f1⟩, ⟨f2⟩], rel=discourse-juxtaposition            -- two sentences = SEQ, not a conjunction
```

**7.38 / 7.39 — `go'i` copies a bridi; indexicals re-resolve per speaker (the C-17 payoff).** `mi klama le zarci .i do go'i`:
```
⟨frame u1/e0/a1/a2⟩
REF Z : kind=const, flavor=le, sort=Obj ; +LE-clause(Z, zarci)
EV  e1 : tense=?, caha=ca'a
PRD p1 : rel=klama, ev=e1, args=[a1, Z, z1, z2, z3], mode=asserted    -- mi = u1.speaker
EV  e2 : tense=?, caha=ca'a
PRD p2 : rel=klama, ev=e2, args=[a2, Z, z1, z2, z3], mode=asserted    -- `do go'i`: copy p1, x1 := u1.audience
REF z1 : kind=const, flavor=zo'e, sort=Obj
REF z2 : kind=const, flavor=zo'e, sort=Obj
REF z3 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : p1
FRM f2 : p2
SEQ s  : items=[⟨f1⟩, ⟨f2⟩], rel=discourse-juxtaposition
```
In 7.39 (`A: mi ba klama …` / `B: mi nelci le si'o mi go'i`), B's explicit `mi` is needed because A's `mi`=`uA.speaker` ≠ B's `mi`=`uB.speaker`; **`ra'o`** on the copy re-points the *copied* inner `mi` from `uA.speaker` to `uB.speaker` — exactly C-17's re-point operation. **7.48** (anaphora across quotations): `la .alis. cusku lu mi go'i li'u` — the `go'i` copies the bridi of the *quoted* `lu mi klama le zarci li'u` (an earlier nested `UTT`), and its `mi` re-resolves to Alice's quoted-utterance speaker; cross-quotation reference stays **within the quoted-utterance stream**, never reaching the narrative outside.

**Other constructs (all shared-id / one-node):**
- **`ma` (7.9)** `ma klama le zarci` → `UTT force=ask, gap=[q1]`, `PAR q1 : sort=Obj, role=ma`, `klama(e1; q1, Z, zo'e…)`. **`mo`** → a `PAR sort=Rel, role=ma` in `rel` position.
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
EV  eb : tense=?, caha=ca'a
PRD pb : rel=blanu, ev=eb, args=[x1, z1], mode=restrictive           -- poi: x1 restricted to the blue one(s)
REF z1 : kind=const, flavor=zo'e, sort=Obj
EV  e1 : tense=?, caha=ca'a
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
EV  es : tense=?, caha=ca'a
PRD ps : rel=srana, ev=es, args=[c1, a1], mode=restrictive           -- pe: c1 pertains to me (= u1.speaker)
FRM (ps attaches to c1; the host clause supplies f1)
```
`po` (specific) → a stronger possession relation; `po'e` (inalienable) → an intrinsic part-relation `R[part-of]`; `ne` (incidental) → `ps` with `mode=incidental`. **`voi` (8.5)** `le voi blanu` (non-veridical restrictive) → `skicu(a1, x1, a2, ⟨ka ce'u blanu⟩) mode=restrictive` (restricted to what the speaker *describes* as blue). **`goi` (8.x)** binds a `ko'a` handle to the head's id (shared id). **`zi'e` (8.4)** → several clause-`PRD`s on one head (each with its own mode). **`vu'o` (8.8)** → the clause attaches to a composite `(c1 ⊕ …)` referent rather than to `c1` alone.

## Chapter 9 — modals (sumtcita = shared-eventuality predications)

**9.28 `mi viska do sepi'o le zunle kanla`** — "I see you with-tool my left eye" (`sepi'o`←`pilno`, ρ **internal** via x3).
```
⟨frame u1/e0/a1/a2⟩
REF k1 : kind=const, flavor=le, sort=Obj ; +LE-clause(k1, [zunle ⋗ kanla])   -- the left eye (tanru)
EV  e1 : tense=?, caha=ca'a
PRD p1 : rel=viska, ev=e1, args=[a1, a2, z1], mode=asserted                   -- I see you
PRD pp : rel=pilno, ev=ep, args=[a1, k1, e1], mode=asserted                   -- sepi'o: a1 uses k1 for PURPOSE e1
EV  ep : tense=?, caha=ca'a
REF z1 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : and(p1, pp)                                                          -- the tool-use is co-asserted; falls under na
```
The seeing event `e1` fills `pilno`'s x3 (purpose) — that *is* the ρ-link; `pp` shares the agent `a1` and references `e1` directly. (`pp` is `mode=asserted`: the tool-use is part of what is claimed; it is not in `f1`'s at-issue *connective* structure but is a co-asserted predication of the same event.)

**9.30 `mi cadzu seka'a la .bratfyd.`** — "I walk with-destination Bradford" (`ka'a`←`klama`, SE selects the destination place).
```
⟨frame u1/e0/a1/a2⟩
REF B : kind=const, flavor=la, sort=Obj ; +cmene-clause(B, "bratfyd")
EV  e1 : tense=?, caha=ca'a
PRD p1 : rel=cadzu, ev=e1, args=[a1, z1], mode=asserted                       -- I walk (surface z1)
PRD pk : rel=klama, ev=ek, args=[a1, B, z2, z3, z4], mode=asserted            -- seka'a: same motion, destination B
EV  ek : tense=?, caha=ca'a
REF z1 : kind=const, flavor=zo'e, sort=Obj
REF z2 : kind=const, flavor=zo'e, sort=Obj
REF z3 : kind=const, flavor=zo'e, sort=Obj
REF z4 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : and(p1, pk)
```
The `klama` predication shares the agent `a1` and (by `R`) the same motion as `cadzu`; SE has put `B` in `klama`'s x2.

**`fi'o kanla` (external ρ) and `do'e` (R alone).** `kanla` has no event place, so the link is an explicit external `R`: `kanla(eye, seer) ∧ R[organ-used-in](eye, e1)`. `do'e` (9.34, `lo nanmu be do'e le berti`) = the bare vague `R[·](le_berti, e_nanmu)` with no tag gismu — fully vague "of".

**Causal sentence connection (9.7) — `ko'a broda .i ri'a bo ko'e brode`** (`ri'a`←`rinka`):
```
⟨frame u1/e0/a1/a2⟩  -- (ko'a, ko'e resolved to referents)
PRD pA : rel=broda, ev=eA, args=[…], mode=asserted ;  FRM fA : pA
PRD pB : rel=brode, ev=eB, args=[…], mode=asserted ;  FRM fB : pB
PRD pc : rel=rinka, ev=ec, args=[eA, eB, z1], mode=asserted    -- event eA causes event eB
REF z1 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : and(fA, fB, pc)
```
(`mu'i`→`mukti`, `ni'i`→`nibli`, `ki'u`→`krinu`, each a relation between the two events.) **`JAI` (9.12)** `mi jai rinka le nu …` → `rel=jai:rinka` raising the abstraction-argument into x1 (`jai gau` raises the agent). **Modal negation (9.13)** = `na`/`naku` scoping the modal `PRD` in `f1`. **`KI` (9.14)** sets a modal as a sticky default copied onto later predications.

## Chapter 10 — tenses (all are `EV` attributes; no numbered places)

**10.4 `mi pu klama le zarci`** — "I went to the store" (past).
```
⟨frame u1/e0/a1/a2⟩
REF s1 : kind=const, flavor=le, sort=Obj ; +LE-clause(s1, zarci)
EV  e1 : tense=pu, dist=?, caha=ca'a                                -- pu: τ(e1) < e0; ZI distance unspecified
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
EV  e1 : tense=pu, extent=whole-past, freq=0, caha=ca'a             -- ze'epu = whole past interval; noroi = 0 times
PRD p1 : rel=klama, ev=e1, args=[a1, s1, z1, z2, z3], mode=asserted
REF z1 : kind=const, flavor=zo'e, sort=Obj
REF z2 : kind=const, flavor=zo'e, sort=Obj
REF z3 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : p1
```
`freq=0` over `extent=whole-past` says it never happened in the past, and (crucially) **nothing about the future** — the occurrences are a frequency property of the one event over its interval, not individuated sub-events.

**Every other tense cmavo is another `EV` field, no new objects:** `ZI` → `dist`; `ZEhA` → `extent`; `FAhA`+`VA` → `place`/`sdist`; `VEhA`/`VIhA`/`MOhI` → `sextent`/`dims`/`motion`; `TAhE` (`ta'e klama`) → `distrib=habitual` (`-nai` flips, e.g. `ru'inai` → `distrib=intermittent`); `ROI` (`ci roi`) → `freq=3`; `ZAhO` (`ba'o klama`) → `aspect=ba'o`; `CAhA` (`ka'e klama`) → `caha=ka'e` (overriding the `ca'a` default). **`KI`** → the `EV` value becomes a sticky default copied onto later `EV`s. **`cu'e`** → `UTT force=ask` with a `PAR role=ma` over the `EV`'s tense field. **Tense as sumtcita** (`ca lo nu broda`) → a `cabna`-style `PRD` relating `e1`'s time to the event's time (a modal anchored to another event). **Tense negation** (`na'e`/`naku` on the tense) → a scalar/contradictory operator on the temporal field.

## Chapter 11 — abstraction (`RFY` objects)

**11.13 `mi nelci le nu mi limna`** — "I like (my) swimming" (`nu` event abstraction as a term).
```
⟨frame u1/e0/a1/a2⟩
EV  en : tense=?, caha=?                                            -- abstraction: no actuality forced
PRD pn : rel=limna, ev=en, args=[a1, z1, z2], mode=asserted
REF z1 : kind=const, flavor=zo'e, sort=Obj
REF z2 : kind=const, flavor=zo'e, sort=Obj
FRM fr : pn
RFY r1 : kind=nu, body=fr, abstracted=[en]                          -- the event (body referenced by id)
REF L1 : kind=const, flavor=le, sort=Obj ; +LE-clause(L1, ⟨the event r1⟩)
EV  e1 : tense=?, caha=ca'a
PRD p1 : rel=nelci, ev=e1, args=[a1, L1, z3], mode=asserted
REF z3 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : p1
```
Aktionsart variants change only `r1.kind`: `mu'e` (point), `pu'u` (process, +stages x2), `zu'o` (activity, +repeated-actions x2), `za'i` (state).

**11.28 `… le ka mi prami ce'u`** — property (one `ce'u`); **two `ce'u`** = a relation (distinct `PAR`s).
```
PAR c1 : sort=Obj, role=ce'u
EV  ek : tense=?, caha=?
PRD pk : rel=prami, ev=ek, args=[a1, c1], mode=asserted
FRM fk : pk
RFY rk : kind=ka, body=fk, abstracted=[c1]                          -- property of being loved by me
```
`le ka ce'u prami ce'u` → `abstracted=[c1, c2]` with `prami(ek; c1, c2)`, c1 and c2 **distinct** (relation abstraction). **`ni` (11.33)** `le ni le pixra cu blanu` → `RFY kind=ni, body=⟨blanu(pixra,…)⟩` denoting a `Quantity` on scale x2. **`jei`** → `RFY kind=jei` (a truth-value). **`si'o`/`li'i`/`su'u`** → those kinds, with `mind=⟨REF⟩` for the mind-relative ones.

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
EV  e1 : tense=?, caha=ca'a
PRD p1 : rel=djuno, ev=e1, args=[a1, D1, z2, z3], mode=asserted      -- I know D1 (subject z2, epistemology z3)
REF z2 : kind=const, flavor=zo'e, sort=Obj
REF z3 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : p1
```
That Frank is a fool follows from `djuno` (factive), **not** from `rd`. **`se du'u`** = `du'u`'s x2 = the **sentence (a `SGN`) expressing `rd`** — used to fill linguistic-behaviour places (`cusku le se du'u …` = "says *that* …", distinct from quoting the exact words `lu … li'u`).

**11.49 `mi djuno le du'u ma kau pu klama le zarci`** — indirect question (`kau` focus).
```
PAR q1 : sort=Obj, role=kau
EV  ek : tense=pu, caha=?
PRD pk : rel=klama, ev=ek, args=[q1, s1, z1, z2, z3], mode=asserted
FRM fd : pk
RFY rd : kind=du'u, body=fd, abstracted=[], focus=[q1]              -- the answer is q1's value
… (le du'u rd → D1; djuno(e1; a1, D1, z, z); frame; s1 = le zarci)
```
**`tu'a` (11.64)** `mi troci tu'a le vorme` → `RFY rt : kind=su'u, body=⟨R[do-with](et; z_agent, V)⟩, abstracted=[et]`, `V = le vorme`; `troci(e1; a1, ⟨le su'u rt⟩, z)`. **`jai`** is the converse, raising the abstraction's argument into x1 at the selbri level (`le jai rinka be le nu do morsi` = "the one who caused your death").

## Chapter 12 — lujvo (atomic relation + `REL` metadata; tanru desugar, lujvo do not)

**12.36 `la .ma([s)] cu dalmikce le gerku`** — "Mary is a vet for the dog" (`dalmikce` = `danlu mikce`, "animal doctor"; asymmetrical place-merge).
```
⟨frame u1/e0/a1/a2⟩
REF M : kind=const, flavor=la, sort=Obj ; +cmene-clause(M, "maris")
REF g1 : kind=const, flavor=le, sort=Obj ; +LE-clause(g1, gerku)
EV  e1 : tense=?, caha=ca'a
PRD p1 : rel=dalmikce, ev=e1, args=[M, g1, z1, z2, z3], mode=asserted   -- ATOMIC rel; args are the merged places
REF z1 : kind=const, flavor=zo'e, sort=Obj                              -- species (d2)
REF z2 : kind=const, flavor=zo'e, sort=Obj                              -- ailment (m3)
REF z3 : kind=const, flavor=zo'e, sort=Obj                              -- treatment (m4)
REL rDM : veljvo=[danlu ⋗ mikce], r=[m1=x1, m2=d1=x2(animal patient), d2=x3, m3=x4, m4=x5],
          places="m1 doctor, m2=d1 animal patient, d2 species, m3 ailment, m4 treatment",
          expansion=⟨mikce(M, g1, z2, z3) ∧ danlu(g1, z1) ∧ R⟩            -- documentation; NOT in any FRM
FRM f1 : p1
```
The lujvo is used as an **atomic `rel`** in `f1`; `REL rDM` records the mechanical destructuring (veljvo, the shared place `m2=d1`, the dropped/kept places, the ordering) as **documentation that never enters any `FRM`**. If `dalmikce` were *unknown* to the audience, `rDM.expansion` (the `[danlu ⋗ mikce]` tanru with a vague `R`) becomes the operative approximation — but predication still proceeds on the bare lujvo.

**Implicit-abstraction lujvo — `ctigau` (`citka gasnu`, "feeder").** `gasnu` (x1 agent brings about event x2) contributes an **event place** filled by an implicit `nu`-abstraction of the seltau:
```
PRD p1 : rel=ctigau, ev=e1, args=[A, F, z1], mode=asserted              -- A feeds F
RFY r2 : kind=nu, body=⟨citka(ec; F, A, z2)⟩, abstracted=[ec]           -- the implicit eating-event
REL rCG : veljvo=[citka ⋗ gasnu], r=[g1=x1 agent, g2=⟨the nu r2⟩, c1=F eater, c2 food],
          note="gasnu's event place x2 is the implicit abstraction r2 — surfaces as an RFY, not a sumti slot"
```
The `nu` is not on the surface but is mandated by `gasnu`'s place structure — a claim-by-place-structure parallel to the `botpi` lid. **`gerzda`** (`gerku zdani`, kennel) is the plain asymmetrical case: `gerzda(z1, d1=z2, …)`, `REL` recording `zdani`'s x2 = the dog. **`zei`-lujvo** (e.g. `xy. zei kantu` "X-ray") — the `REL.veljvo` carries the **full C-18 destructuring of its constituents' meanings** (here a lerfu `SGN` + `kantu`), **not** the quoted words; the constituents contribute relations, not text.

## Chapter 13 — attitudinals & evidentials (the `DSP` tier; no truth value, never in `FRM`)

CLL 13.2 is explicit: attitudinals "make no claim… have no truth value, nor do they directly affect the truth value of a bridi that they modify." So a `DSP` never enters `f1` and never changes whether the host is true. The pure-emotion vs propositional-attitude split is then about **where the host bridi sits**, not about its truth value.

**13.x `.ui mi klama le zarci`** — "I'm going to the store — yay!" (**pure emotion**: host stays asserted).
```
⟨frame u1/e0/a1/a2⟩
REF s1 : kind=const, flavor=le, sort=Obj ; +LE-clause(s1, zarci)
EV  e1 : tense=?, caha=ca'a
PRD p1 : rel=klama, ev=e1, args=[a1, s1, z1, z2, z3], mode=asserted
REF z1 : kind=const, flavor=zo'e, sort=Obj
REF z2 : kind=const, flavor=zo'e, sort=Obj
REF z3 : kind=const, flavor=zo'e, sort=Obj
DSP d1 : kind=emotion, rel=ui[happiness], experiencer=a1, target=e1, anchor=e0, polarity=+, intensity=?
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
DSP d1 : kind=prop-attitude, rel=au[desire], experiencer=a1, target=rp, anchor=e0, intensity=?
FRM f1 : ⟨empty⟩                                                      -- NO at-issue real-world claim
```
The going is **not** in any asserted real-world `f1` — `rp` characterizes the hypothetical world `d1` reacts to (CLL: "an internal hypothetical world… distinct from the world as it really is"; from "I hope George wins" you may conclude nothing about George). The DSP did not make the going false; it relocated it. (`.ei` obligation, `.ai` intent, `.e'a` permission, `.e'u` suggestion behave the same.) To actually assert the going you must split it into its own sentence — the diagnostic that propositional attitudes do not assert their host.

**Evidentials set the host's force (gismu-linked):**
- **`za'a` (`zgana`, "I observe")** — host **asserted**, source-marked: `DSP d : kind=evidential, rel=zgana, experiencer=a1, target=⟨the proposition⟩` **and** the bridi stays in `f1` (asserted, indisputable-from-observation).
- **`ti'e` (`tirna`, hearsay) / `ru'a` (`sruma`, assumption)** — host **not** speaker-asserted: bridi reified into the gismu's `du'u` (sits as the evidential's target), `f1` empty.
- **`ca'e` (define/performative)** — host **made true by the utterance**: `PRD … mode=performative` in `f1` (the `ca'e`/definition case).

**Modifiers (no truth effect):** `dai` → `experiencer` shifts to a non-speaker / empathic referent; `pei` → the emotion-type or intensity becomes a `PAR role=ma` (attitude question, `UTT force=ask` over the DSP field); `ge'e` → explicit null DSP; `bu'o`/`bu'oi`/`bu'onai` → `phase=start|continue|end` (the displayed-tier `ZAhO`, distinct from an asserted `ba'o prami` claim); `CAI` (`cai`/`sai`/`ru'e`/`cu'i`) → `intensity`/`polarity`. **Insincerity** (feeling `.ui` you don't feel) = **infelicity, not falsity** — there is no truth value to negate. **Scope by placement:** sentence-initial/post-selbri ⇒ `target=e1`; after a sumti ⇒ `target` = that sumti.

## Chapter 14 — connectives (the `FRM` layer; the locus fixes sharing)

**14.49/14.50 `mi klama le zarci gi'e nelci la .djan.`** — compound bridi sharing x1.
```
⟨frame u1/e0/a1/a2⟩
REF Z : kind=const, flavor=le, sort=Obj ; +LE-clause(Z, zarci)
REF J : kind=const, flavor=la, sort=Obj ; +cmene-clause(J, "djan")
EV  eA : tense=?, caha=ca'a
PRD pA : rel=klama, ev=eA, args=[a1, Z, z1, z2, z3], mode=asserted     -- a1 shared (the one explicit x1)
EV  eB : tense=?, caha=ca'a
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
EV  eA : tense=?, caha=ca'a
PRD pA : rel=klama, ev=eA, args=[zA, NY, Ph, z1, z2], mode=asserted    -- own x1 zA, own route z1, means z2
EV  eB : tense=?, caha=ca'a
PRD pB : rel=klama, ev=eB, args=[zB, NY, Ro, z3, z4], mode=asserted    -- DISTINCT x1 zB, route z3, means z4
REF zA : kind=const, flavor=zo'e, sort=Obj
REF zB : kind=const, flavor=zo'e, sort=Obj
REF z1 : kind=const, flavor=zo'e, sort=Obj
REF z2 : kind=const, flavor=zo'e, sort=Obj
REF z3 : kind=const, flavor=zo'e, sort=Obj
REF z4 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : and(pA, pB)
```
Writing `da` in x1 (14.59) replaces `zA`/`zB` with **one** bound `var` (`EX(X, and(pA[zA:=X], pB[zB:=X]))`) — forcing x1 identity while leaving the routes distinct. This is the whole "elided place = its own `zo'e`, never silently shared" discipline, and why **`X .e Y cu P ≠ X cu P .ije Y cu P`** in general: the `.e` (sumti) form is **one** predication whose other elided places are shared ids, while the `.ije` expansion is **two** predications each with its own `zo'e`.

**14.x sumti connection `do .e mi`** — one shared predication, distributing.
```
PRD p1 : rel=R…, ev=e1, args=[a2, …shared zo'e…], mode=asserted        -- the `do` instance
PRD p2 : rel=R…, ev=e2, args=[a1, …SAME shared zo'e ids…], mode=asserted -- the `mi` instance, differing ONLY in x1
FRM f1 : and(p1, p2)
```
The two predications **share** every elided place (same `zo'e` ids), differing only in the connected argument — contrast the fully-independent `.ije` form above.

**Realization algebra (one `FRM` node per connection):** `.a`→`or`, `.e`→`and`, `.o`→`iff`, `.u`→ first asserted + second `mode=inert`; `na`/`nai`/`se` transform structurally — `na.a`→`or(not p,q)` (truth-functionally implication), `.enai`→`and(p, not q)`, `se.u`→`inert(p), assert(q)`. **Connective question `ji`** (`do ji mi`) → `PAR role=ma, tier=connective` at the connective node, `UTT force=ask`. **Forethought `ge…gi`** = same `FRM` nodes, prefix order. **Termset `nu'i…nu'u`** = parallel connection of several places. **Non-logical `joi`/`ce`/`ce'o`/`fa'u` (14.15)** do **not** enter `FRM`: `joi`→a `gunma` composite, `ce`→a `selcmi` set, `ce'o`→an ordered sequence, `fa'u`→a respectively-pairing — all **composite referents** fed to one predication.

## Chapters 15–16 — negation & quantifier scope (reference-by-id)

**15.2 `mi na klama le zarci`** — "It is not the case that I go to the store" (contradictory bridi negation).
```
⟨frame u1/e0/a1/a2⟩
REF s1 : kind=const, flavor=le, sort=Obj ; +LE-clause(s1, zarci)
EV  e1 : tense=?, caha=ca'a
PRD p1 : rel=klama, ev=e1, args=[a1, s1, z1, z2, z3], mode=asserted
REF z1 : kind=const, flavor=zo'e, sort=Obj
REF z2 : kind=const, flavor=zo'e, sort=Obj
REF z3 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : not(p1)
```
**`na'e` (scalar)** `mi na'e klama` → `rel=na'e:klama` (a *positive* claim: I relate to the store by some other motion-relation). **`na'i` (metalinguistic)** → `DSP d : kind=metalinguistic, rel=na'i, target=⟨the bridi/term⟩` (flags a false presupposition; no truth value, not a `not`).

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
**`no da` (16.x)** `no da broda` → `NO(v, broda(v))` = `not(EX(v, broda(v)))`. **`da`/`de`/`di` ordering** in one prenex = nested quantifier nodes in surface order (`EX(X, ALL(Y, …))` ≠ `ALL(Y, EX(X, …))` — the ∃∀/∀∃ distinction the FRM nodes exist to preserve).

## Chapters 17–18 — letterals (SGN handles) & mekso (MEX)

**17.x letteral as pro-sumti — `lo gerku ... gy. ...`** ("the dog … it[g] …"). The lerfu is a handle; once anaphora is resolved it is a `REF` carrying that handle.
```
⟨frame u1/e0/a1/a2⟩
REF g1 : kind=const, flavor=lo, sort=Obj, handle=gy.                 -- gy. (initial of gerku) reused for this referent
… (g1 then appears by shared id wherever `gy.` recurs) …
SGN ly : kind=lerfu, source=GLYPH⟨"g"⟩, denotes=Latin-g              -- the lerfu word itself, qua sign
```
The three **uses** of a lerfu, all the handle idea: **character** (`me'o gy.` = the letter as an object → a `SGN`-referent); **pro-sumti** (above — a `REF` with `handle=gy.`, by shared id; a lerfu *string* = **one** referent); **mekso variable** (`PAR`/`var` over `Quantity` with `handle=`). The three **source types**: `GLYPH⟨"g"⟩` (symbolic — the glyph text *is* the denotation; strings are concatenation, e.g. acronyms); `REL⟨…⟩` evocative/non-veridical (`denpa bu` = the dot *because* `denpa`=pause — a content word *is* its relation; a `zei`/rafsi lerfu carries its C-18 destructuring; metaphorical, so never asserted); `QUOTATION` (`zo`/`zoi` — a real opaque quotation `SGN`).

**18.x value vs sign — `li vo su'i re du li xa`** ("4 + 2 = 6", values) and `me'o vo su'i re` (the unevaluated expression).
```
⟨frame u1/e0/a1/a2⟩
MEX m1 : op=su'i, operands=[4, 2]                                    -- the mekso tree 4 + 2
REF L1 : kind=const, sort=Quantity, flavor=li ; value=⟨eval m1⟩      -- li (vo su'i re): the VALUE 6
REF L2 : kind=const, sort=Quantity, flavor=li ; value=6             -- li xa
EV  e1 : tense=?, caha=ca'a
PRD p1 : rel=du, ev=e1, args=[L1, L2], mode=asserted                 -- the two values are identical
FRM f1 : p1
```
`me'o vo su'i re` instead yields `SGN q : kind=grammatical, text="vo su'i re"` (the **unevaluated** expression as a sign — `me'o` ≠ `li`, so `me'o vo su'i re` ≠ `li xa` even though the value would be 6). **MOI relations** (number→relation): `mei` (`lo ci mei` = a mass/set of 3, `mei(x, 3)`), `moi` (`la .alis. cu ralju lo'i prenu vo moi` → `moi(alis, set, 4, rule)` = 4th), `si'e` (portion), `cu'o` (probability). **`nu'a` (18.x)** `nu'a su'i` converts the operator `su'i` back to a **relation** (a selbri: x1 is the sum of x2, x3, …). Internally operators are VUhU (`su'i`=+, `vu'u`=−, `pi'i`=×, `te'a`=^); operands are numbers, `li`-quantities, lerfu variables (with `handle=`), special numbers (`pi`=π, `te'o`=e, `ka'o`=i), and `xi`-subscripts. Infix/forethought/RPN are surface orderings of the one `MEX` tree. **Quantifier use** (`re lo broda`, a number as a sumti-prefix) feeds `CARD`/`PA` into a quantifier `FRM`-node (ch. 6/16), the fourth and last door from `MEX` into the core.

## Chapter 19 — text structure (SEQ, quotation, parentheticals)

**19.1 `mi klama le zarci .i do cadzu le bisli`** — two sentences, vague relationship (a `SEQ`, **not** a conjunction).
```
UTT u1 : force=assert, content=f1, ev=e0, speaker=a1, audience=a2
REF a1 : kind=const, sort=Obj
REF a2 : kind=const, sort=Obj
EV  e0 : tense=now, caha=ca'a
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
SEQ s  : items=[⟨f1⟩, ⟨fb⟩], rel=discourse-juxtaposition            -- TRUTH-VALUELESS; relationship left vague
```
`.i` is **never** a truth function; the two bridi each keep their own truth value and import. **`.ije`** instead makes one `FRM` connective (`and(f1, fb)`); **`.i bo`** / **`tu'e…tu'u`** group `SEQ` items more tightly (precedence only, analogous to `ke…ke'e`); **NIhO** opens a higher-level `SEQ` (paragraph).

**19.4 structured vs opaque quotation.** `la .djan. cusku lu mi klama li'u` — `lu…li'u` is a **nested `UTT`** with reachable referents.
```
⟨frame u1/e0/a1/a2⟩
REF J : kind=const, flavor=la, sort=Obj ; +cmene-clause(J, "djan")
UTT u2 : force=mentioned, content=f2, ev=e2, speaker=J, audience=z1   -- the quoted utterance; its OWN roles
PRD p2 : rel=klama, ev=ek, args=[J2, Zq, …], mode=asserted           -- inner `mi` = u2.speaker (= J, by content)
REF J2 : kind=const, indexical=⟨u2.speaker⟩, sort=Obj
REF Zq : kind=const, flavor=le, sort=Obj ; +LE-clause(Zq, zarci)
FRM f2 : p2
REF z1 : kind=const, flavor=zo'e, sort=Obj
EV  e1 : tense=?, caha=ca'a
PRD p1 : rel=cusku, ev=e1, args=[J, u2, z2, z3], mode=asserted        -- John expresses the utterance u2
REF z2 : kind=const, flavor=zo'e, sort=Obj
REF z3 : kind=const, flavor=zo'e, sort=Obj
FRM f1 : p1
```
Inner `mi`→`u2.speaker` (C-17); referents inside `u2` are reachable from outside by shared id (the 7.48 cross-quotation case). **Opaque** `lo'u mi klama le'u` / `zo klama` / `zoi gy. ... gy.` → a sealed `SGN kind=quotation` (token sequence, **no** internal `UTT`, no reachable referents). **`la'o .X. ... .X.`** = a name from foreign text (`la me zoi`). **`la'e`/`lu'e`** = the reference↔referent shifts (the `zo .bab.` word / `la .bab.` named thing / `la'e zo .bab.` referent triad).

**19.x topic-comment `lo cukta zo'u mi pinxe`** — vague link (genuinely underspecified, exposed not fabricated).
```
⟨frame u1/e0/a1/a2⟩
REF C : kind=const, flavor=lo, sort=Obj ; +PRD cukta(C,…) incidental   -- the topic
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

See the closing summary in the accompanying message. In brief: (1) **tanru** use one uniform schema — tertau asserted + seltau as a reified kind `⟨ka ce'u S⟩` linked by the vague `R`, asserting neither `S(x)` nor a concrete seltau referent (revised under C-26 from the earlier two-sub-case form, which over-committed); (2) **asserted modal predications** conjoined into content (fall under `na`); (3) **restricted universals** carry categorical domain-nonemptiness import (CLL 16.8), distinct from the import-free "any". A later cross-check against jbotci `tersmu`'s Lean prelude (C-26) corroborated the model broadly and prompted the uniform tanru schema plus the `Proposition`/`TruthValue` sorts and the `gi'e`/`.e` sharing fix; no other definitions required revision; all remaining chapters validated against CLL as-is.
