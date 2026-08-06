# Human-readable smusni S-expression samples

These samples exercise the version-0 rules in [the specification](spec.md).
They are design specimens, not golden renderer expectations. A block labelled
“fragment” is intended to appear inside a complete document; every other block
is a complete `Smusni` datum.

The English glosses explain the represented distinction and are not claimed as
word-for-word translations.

## 1. Predication, closure, and assertion

`mi klama` assembles predicate content, closes its remaining defaultable places,
and then explicitly constructs an assertion act:

```lisp
(Smusni 0
  (Assert
    (klama Speaker)))
```

The missing destination, origin, route, and means are distinct contextual
defaults. They are not printed as repeated `zo'e` nodes. The event is local and
unshared, so `Close` and the event binder are both omitted by the canonical
rules.

Filling every numbered place would still not assert:

```lisp
(Smusni 0
  (Mention
    (klama Speaker This Speaker This This)))
```

Here the saturated predicate term is mentioned as a value. It is not coerced to
`Content` because `Mention` is polymorphic.

## 2. Current places, computed `At`, and `zi'o`

A literal skip uses a keyword. The plain value after `:2` fills current x3:

```lisp
(Smusni 0
  (Assert
    (klama :2 This Yonder)))
```

An actual place question uses computed `At` and no later plain operand:

```lisp
(Smusni 0
  (Ask
    (OpenQ
      (λ (($p (PlaceOf klama (Referents Entity))))
        (klama :5 This (At $p This))))))
```

Two computed fills whose candidate domains overlap do not print as two `At`
forms. The smallest content position falls back because answer substitution
could assign both values to the same place. Its registered local reason is
`smusni.fallback.at.overlapping-candidates`; the fallback preserves the actual
smallest reachable raw model owner rather than inventing a
`ComputedPlaceAssignment` model type. Section 20 shows the complete raw syntax.

`zi'o` removes a current numbered place. Surviving labels keep their visible
numbers and plain traversal skips the hole:

```lisp
(Smusni 0
  (Assert
    ((DropPlace klama 3)
      Speaker
      This
      This
      This)))
```

## 3. Source place conversion is eliminated

For source `ti se klama mi`, source x1 `This` fills base `klama` x2 and source
x2 `Speaker` fills base x1. The base order needs no labels, and there is no
`Se` node:

```lisp
(Smusni 0
  (Assert
    (klama Speaker This)))
```

A converted property is an ordinary lambda over the permuted base fill:

```lisp
(Smusni 0
  (Let (($destination-property
          (Fn ((Referents Entity)) Content)
          (λ (($x (Referents Entity)))
            (klama :2 $x))))
    (Mention $destination-property)))
```

If the converted binary relation itself escapes into an ordered function place,
it eta-expands:

```lisp
(λ (($new-x1 (Referents Entity))
    ($new-x2 (Referents Entity)))
  (tavla $new-x2 $new-x1))
```

That last block is a fragment. A place expecting a labelled `PredTerm` rather
than the displayed `Fn` cannot consume it and uses typed fallback.

Tanru and scalar relation formers remain relational values rather than
grammar-shaped records. Their applications follow the effective row of the
head relation:

```lisp
(Smusni 0
  (Assert
    ((Tanru sutra klama)
      Speaker
      This)))
```

```lisp
(Smusni 0
  (Assert
    ((Scalar OtherThan melbi)
      This)))
```

These specimens require the corresponding graph relation and scalar reading;
the renderer does not infer either from an English gloss.

## 4. Modals are predicates joined by `Joi`

A `sepi'o` modal shares the event of the main clause with a `pilno` predication:

```lisp
(Smusni 0
  (Assert
    (∃
      (λ (($e Eventuality))
        (Joi
          (klama Speaker This :Eventuality $e)
          (pilno :2 This $e))))))
```

Direct `fi'o pilno` uses the place map represented by that tag rather than
assuming the `sepi'o` map:

```lisp
(Smusni 0
  (Assert
    (∃
      (λ (($e Eventuality))
        (Joi
          (klama Speaker This :Eventuality $e)
          (pilno Speaker This $e))))))
```

A compound, negated tag retains its connector and negation:

```lisp
(Smusni 0
  (Assert
    (∃
      (λ (($e Eventuality))
        (Joi
          (klama Speaker :Eventuality $e)
          (∨
            (¬ (pilno :2 This $e))
            (purci $e Now)))))))
```

There is no modal-valued `At`; arguments inside each modal predicate are normal
predicate fills.

A tense uses the same ordinary-predicate-plus-`Joi` machinery. Here `pu`
reduces to the standard `fi'o se purci` analysis, so the event is x1 of
`purci` and the utterance time is x2:

```lisp
(Smusni 0
  (Assert
    (∃
      (λ (($e Eventuality))
        (Joi
          (citka Speaker :Eventuality $e)
          (purci $e Now))))))
```

`mo'i ri'u` is not a generic motion flag. Once the graph identifies the motion
event and mover, the axial reduction uses the transparent `MotionVector`
helper, whose definition projects `muvdu` by deleting only the unrepresented
route place:

```lisp
(Smusni 0
  (Assert
    (∃
      (λ (($e Eventuality))
        (Joi
          (cadzu Speaker :Eventuality $e)
          (MotionVector
            $e
            Speaker
            (λ (($to (Referents Entity))
                ($from (Referents Entity)))
              (pritu $to $from Speaker))))))))
```

This specimen represents the walking event itself as the motion and identifies
the walker as its mover. Neither follows mechanically from lexical x1: a
separate concurrent motion uses its own event plus `cabna` only when that row is
verified, and a result-dependent reading likewise requires its represented
result relation. Without the corresponding exact row, either reading uses typed
fallback. Other `mo'i` directions retain and delete different lexical places,
as specified by their reduction rows.

When a verified heading reduction represents no destination, origin, or route,
it can project exactly the surviving mover place and relate the same motion to
a first-class direction:

```lisp
(∧
  ((DropPlace (DropPlace (DropPlace muvdu 2) 3) 4)
    $mover
    :Eventuality $motion)
  (farna $direction $motion $frame))
```

That block is a fragment with graph-bound operands. The three deletions are
semantic `zi'o`, not abbreviated contextual omission.

Source `ka'e` uses the transparent capability helper. Its host property keeps
the candidate bearer and possible event explicit:

```lisp
(Smusni 0
  (Assert
    (InnatelyCapable
      Speaker
      (λ (($candidate (Referents Entity))
          ($possible (Referents Eventuality)))
        (klama $candidate This :Eventuality $possible)))))
```

The helper deletes no `kakne` place: its condition place remains an ordinary
contextual argument. Actuality is a separate predicate, not assertion force:

```lisp
(Smusni 0
  (Assert
    (∃
      (λ (($e Eventuality))
        (Joi
          (citka Speaker :Eventuality $e)
          (fasnu $e))))))
```

An aspect whose boundary/checkpoint relation has not yet been verified remains
local fallback rather than an English-gloss constructor or an approximate
`cfari` predication. Its registered local reason is
`smusni.fallback.tag.coha.unverified-boundary`; its raw value is the actual
reachable owner containing the inline aspect value, not a synthetic
`EventContour` object.

A repeated tense is a path rather than two unrelated facets. This sample
assumes one graph locus with a joint two-parameter existential; the nested-`∃`
specimen in the specification represents nested binder identity rather than a
second canonical spelling of this graph:

```lisp
(Smusni 0
  (Assert
    (∃
      (λ (($event Eventuality)
          ($middle Eventuality))
        (Joi
          (citka Speaker :Eventuality $event)
          (purci $middle Now)
          (purci $event $middle))))))
```

Sticky `ki` is not a discourse-state node in the output. For source
`puki ... .i ...`, the resolved `pu` relation is simply repeated at each
inheriting event:

```lisp
(Smusni 0
  (Do
    (Assert
      (∃
        (λ (($first Eventuality))
          (Joi
            (klama Speaker This :Eventuality $first)
            (purci $first Now)))))
    (Assert
      (∃
        (λ (($second Eventuality))
          (Joi
            (batci This That :Eventuality $second)
            (purci $second Now)))))))
```

A later explicit tense composes onto the inherited path in source order, just
as an explicitly repeated tense would.

This specimen shares only the deictic `Now`. If the graph instead owns one
sticky reference-time anchor distinct from `Now`, that anchor is bound once
outside `Do` with `Let` when inert or `Bind` when computed, and both repeated
`purci` applications use the same variable.

Version 0 does not pretend that a graph-owned noncurrent deictic ground is the
current speech situation. Until that registered reduction exists, the smallest
referential value falls back with reason
`smusni.fallback.deictic.noncurrent-ground`. The raw encoding uses a
`RawRecord` for an inline deictic-reference value and an `Object`/`Ref` only for
the real graph-owned ground referent; it never promotes either into an invented
semantic-object type.

## 5. Explicit `Close` for shared predicate terms

An inline known-row predicate closes implicitly at `Assert`:

```lisp
(Smusni 0
  (Assert
    (melbi This)))
```

The version-0 lexical row treats `melbi` x3 and x4 as ordinary
`Referents<Entity>` roles for the conceptual aspect/property and aesthetic
standard, not as function-valued places. The dictionary's `(ka)` gloss does
not itself impose a function type, so contextual closure of these roles is
legal under that row.

The same predicate term prints `Close` once it has identity of its own:

```lisp
(Smusni 0
  (Let (($description
          (PredTerm
            (Row
              (2 (Referents Entity))
              (3 (Referents Entity))
              (4 (Referents Entity))
              (Eventuality (Referents Eventuality))))
          (melbi This)))
    (Do
      (Mention $description)
      (Assert (Close $description)))))
```

A shared property may execute its lexical `Close` more than once without
turning one omitted source place into several contextual choices. In this
specimen the graph marks those omissions `Fixed`; each omitted place below has
one lexical closure-site identity reused by both applications during this
performance:

```lisp
(Smusni 0
  (Let (($goer-property (Fn ((Referents Entity)) Content)
          (λ (($x (Referents Entity)))
            (klama $x))))
    (Assert
      (∧
        ($goer-property Speaker)
        ($goer-property This)))))
```

The two applications substitute different x1 values; they do not mint new
identities for klama's omitted destination, origin, route, or means sites.

A nondefaultable higher-order gap cannot be silently closed. It uses local
fallback with reason `smusni.fallback.close.nondefaultable-place`, rooted in the
actual reachable semantic-model owner; `PredicateTerm` is notation IR and is
not fabricated as a raw model object.

An elided value whose graph scope may depend on `$p` cannot disappear into
`Close`; its dependency set is explicit. This is a fragment inside the binder
for `$p`:

```lisp
(Bind (($destination (Referents Entity) (Context $p)))
  (Assert
    (klama Speaker $destination)))
```

## 6. Dynamic accessibility across connectives

A successful conjunction can introduce a reference for its right operand and
the later discourse:

```lisp
(Smusni 0
  (Bind (($cat (Referents Entity)
          (Refer
            (λ (($x (Referents Entity)))
              (mlatu $x)))))
    (Do
      (Assert
        (∧
          (jbena $cat)
          (blabi $cat)))
      (Assert
        (ciska $cat)))))
```

This specimen assumes the graph explicitly owns one cross-act identity. Its
`Bind` is at that legal common host, so lexical scope and dynamic accessibility
both include the two acts; ordinary default raising would stop at a force
boundary.

An ordinary xorlo reference is fixed for the force segment, so it scopes
outside visible negation:

```lisp
(Smusni 0
  (Bind (($cat (Referents Entity)
          (Refer
            (λ (($x (Referents Entity)))
              (mlatu $x)))))
    (Assert
      (¬
        (jbena $cat)))))
```

By contrast, the individual variable introduced inside this negated
quantifier is both lexically and dynamically local:

```lisp
(Smusni 0
  (Assert
    (¬
      (∃
        (λ (($x Entity))
          (∧
            (mlatu $x)
            (jbena $x)))))))
```

An implication passes the successful antecedent context to its consequent.
When the graph says both operands use the same identity, one lexical binder
must span both. This donkey-style normalization uses an explicit quantifier:

```lisp
(Smusni 0
  (Assert
    (∀
      (λ (($x Entity))
        (→
          (∧
            (mlatu $x)
            (jbena $x))
          (ciska $x))))))
```

This is not a same-type `Context` lookup. If the consequent instead contains a
genuinely new contextual choice, it receives its own `Bind`; if coreference is
required but the graph does not identify the antecedent, the affected
conditional uses typed fallback.

## 7. `lo`, `le`, and `la` are compositional references

`lo mlatu` uses the veridical property directly:

```lisp
(Smusni 0
  (Bind (($cat (Referents Entity)
          (Refer
            (λ (($x (Referents Entity)))
              (mlatu $x)))))
    (Assert
      (blabi $cat))))
```

There is no `Lo` constructor.

A `le` description composes the speaker-description relation rather than
asserting that the referent is extensionally a cat:

```lisp
(Smusni 0
  (Bind (($described (Referents Entity)
          (Refer
            (λ (($x (Referents Entity)))
              (skicu Speaker $x Audience
                (λ (($y (Referents Entity)))
                  (mlatu $y)))))))
    (Assert
      (blabi $described))))
```

A name description uses a sign and the graph's naming relation:

```lisp
(Smusni 0
  (Bind (($named (Referents Entity)
          (Refer
            (λ (($x (Referents Entity)))
              (Named "alis" $x)))))
    (Assert
      (klama $named))))
```

`Typical` is retained because typical reference is not ordinary unique or
indefinite selection:

```lisp
(Bind (($typical-cat (Referents Entity)
        (Typical
          (λ (($x (Referents Entity)))
            (mlatu $x)))))
  ⟦body⟧)
```

The last block is a schema fragment; `⟦body⟧` is metanotation, not output
syntax.

A speaker-owned stereotype keeps its describer as an ordinary operand rather
than hiding it in the constructor:

```lisp
(Bind (($stereotypical-cat (Referents Entity)
        (Stereotypical
          Speaker
          (λ (($x (Referents Entity)))
            (mlatu $x)))))
  ⟦body⟧)
```

This is also a schema fragment.

## 8. Relative clauses are ordinary composition

Source `lo mlatu poi blabi` contributes both veridical predicates inside one
reference property:

```lisp
(Smusni 0
  (Bind (($white-cat (Referents Entity)
          (Refer
            (λ (($x (Referents Entity)))
              (∧
                (mlatu $x)
                (blabi $x))))))
    (Assert
      (jbena $white-cat))))
```

An explicit outer quantifier creates a semantic placement boundary. In
`re lo mu plise poi xunre`, the inner `poi` helps define the five-member base
reference before the outer selection. In `re lo mu plise ku poi xunre`, the
base is five apples and the outside-`ku` clause instead joins the outer
quantifier restriction. Schematically, the latter has this shape (with the
source-licensed singular apple basis written explicitly):

```lisp
(Smusni 0
  (Bind (($apples (Referents Entity)
          (Refer
            (λ (($r (Referents Entity)))
              (∧
                (plise $r)
                (= (Card
                    (SetOf
                      (λ (($x Entity))
                        (∧
                          (plise $x)
                          (Among $x $r)))))
                   5))))))
    (Assert
      ((Exactly 2
         (λ (($x Entity))
           (∧
             (plise $x)
             (Among $x $apples)
             (xunre $x))))
       (λ (($x Entity))
         (lebna Speaker $x))))))
```

Here `$apples` is the five-apple base reference before `Exactly 2` performs the
outer selection. Moving `xunre` into its property would incorrectly say that
all five base apples are red.

When a graph instead represents an outer `poi` as selection of the greatest
satisfying subreference, no `Relative` record or maximality primitive is
needed. For previously computed `$demonstrators` and `$building`, the clause
property is bound once so its omitted-place closure identity is shared by both
applications:

```lisp
(Let (($surrounds (Fn ((Referents Entity)) Content)
        (λ (($r (Referents Entity)))
          (sruri $r $building))))
  (λ (($surrounders (Referents Entity)))
    (∧
      (Among $surrounders $demonstrators)
      ($surrounds $surrounders)
      (∀
        (λ (($candidate (Referents Entity)))
          (→
            (∧
              (Among $candidate $demonstrators)
              ($surrounds $candidate))
            (Among $candidate $surrounders)))))))
```

This final block is a fragment: the whole `Let` form evaluates to the reference
property to be supplied to `Refer`. It preserves one collective `sruri`
property of the selected plural reference; replacing it with
`sruri $x $building` inside a
singular counting restriction is a different reading.

A source `le gerku voi blabi` composes its one description property with the
transparent prelude helper `DescribedAs` rather than asserting whiteness. The
helper's normative definition removes `skicu` x3; no audience is fabricated
for the `voi` relation:

```lisp
(Smusni 0
  (Bind (($thing (Referents Entity)
          (Refer
            (λ (($x (Referents Entity)))
              (∧
                (skicu Speaker $x Audience
                  (λ (($y (Referents Entity)))
                    (gerku $y)))
                (DescribedAs Speaker $x
                  (λ (($y (Referents Entity)))
                    (blabi $y))))))))
    (Assert
      (jbena $thing))))
```

In `lo gerku noi blabi cu melbi`, the main beauty content is the first operand
and the white relative clause is the supplementary second operand:

```lisp
(Smusni 0
  (Bind (($dog (Referents Entity)
          (Refer
            (λ (($x (Referents Entity)))
              (gerku $x)))))
    (Assert
      (Supplement
        (melbi $dog)
        (blabi $dog)))))
```

The same force-segment handler keeps the side commitment outside transparent
negation and question force. Under the preceding `$dog` binding, the relevant
handler-anchored act fragments are:

```lisp
(Assert
  (Supplement
    (¬ (melbi $dog))
    (blabi $dog)))
```

```lisp
(Ask
  (Polar
    (Supplement
      (melbi $dog)
      (blabi $dog))))
```

In the first, only beauty is negated. In the second, only beauty is questioned.
Whiteness is committed once as supplementary content in both cases. A graph
which represents a genuinely local or conditional supplement names that
different handler explicitly. Canonical output raises a projective supplement
to the outermost type-correct position inside that handler, so the equivalent
lower spelling `(¬ (Supplement (melbi $dog) (blabi $dog)))` does not print.

Multiple clauses retain their connector rather than a list of `Relative`
records:

```lisp
(λ (($x (Referents Entity)))
  (∧
    (gerku $x)
    (∨
      (blabi $x)
      (bunre $x))
    (citka $x $food)))
```

A graph-owned `goi` alias becomes identity sharing:

```lisp
(Let (($friend (Referents Entity) $antecedent))
  (Assert
    (tavla Speaker $friend)))
```

The final two blocks are fragments.

## 9. Generalized quantification and importing universals

Source `ro` uses the importing `Every` constructor:

```lisp
(Smusni 0
  (Assert
    ((Every
       (λ (($x Entity))
         (mlatu $x)))
     (λ (($x Entity))
       (jbena $x)))))
```

Its effect-level expansion is:

```lisp
(Presuppose
  (∃
    (λ (($x Entity))
      (mlatu $x)))
  (∀
    (λ (($x Entity))
      (→
        (mlatu $x)
        (jbena $x)))))
```

The expansion is a fragment and shows why the nonemptiness commitment projects
through an outer negation. There is no `(Import Projective)` record and no
unstated import choice.

The other generalized-quantifier helpers have the same restriction-then-scope
application shape. Existential and negative examples are:

```lisp
(Smusni 0
  (Assert
    ((Some
       (λ (($x Entity))
         (gerku $x)))
     (λ (($x Entity))
       (bajra $x)))))
```

```lisp
(Smusni 0
  (Assert
    ((No
       (λ (($x Entity))
         (gerku $x)))
     (λ (($x Entity))
       (batci $x Speaker)))))
```

A lower-bound cardinal helper additionally requires a pure restriction, a pure
nuclear scope, and the registered singular counting basis:

```lisp
(Smusni 0
  (Assert
    ((AtLeast 2
       (λ (($x Entity))
         (gerku $x)))
     (λ (($x Entity))
       (bajra $x)))))
```

`AtMost`, `MoreThan`, and `FewerThan` differ only in their registered comparison
operator and retain the same purity and counting-basis gate.

The mathematical nonimporting universal remains available directly:

```lisp
(Smusni 0
  (Assert
    (∀
      (λ (($x Entity))
        (→
          (mlatu $x)
          (jbena $x))))))
```

## 10. Sets, plurality, and exact cardinality

An outer exact count is ordinary cardinality of singular satisfiers:

```lisp
(Smusni 0
  (Assert
    (= (Card
        (SetOf
          (λ (($x Entity))
            (∧
              (gerku $x)
              (bajra $x)))))
       3)))
```

One fixed plural reference can instead be constrained through a singular-member
set without identifying the reference with that set:

```lisp
(Smusni 0
  (Bind (($dogs (Referents Entity)
          (Refer
            (λ (($r (Referents Entity)))
              (∧
                (gerku $r)
                (= (Card
                    (SetOf
                      (λ (($x Entity))
                        (∧
                          (gerku $x)
                          (Among $x $r)))))
                   3))))))
    (Assert
      (bajra $dogs))))
```

`Combine` is plural reference formation, not set union:

```lisp
(Assert
  (tavla Speaker
    (Combine $alis $bob)))
```

`ce'o` constructs one referent whose value is an ordered list of reference
values, so the list is singleton-lifted for ordinary sumti use:

```lisp
(Singleton (List $alis $bob $carol))
```

The last two blocks are fragments.

Set and group gadri refer to ordinary set/group objects through lexical
relations; they do not turn the base reference into a mathematical set by
coercion. For `lo'i gerku`:

```lisp
(Smusni 0
  (Bind (($base (Referents Entity)
          (Refer
            (λ (($x (Referents Entity)))
              (gerku $x)))))
    (Bind (($sets (Referents (Set Entity))
            (Refer
              (λ (($x (Referents (Set Entity))))
                (selcmi $x $base)))))
      (Mention $sets))))
```

The parallel `loi gerku` shape changes only the outer object sort and lexical
relation:

```lisp
(Smusni 0
  (Bind (($base (Referents Entity)
          (Refer
            (λ (($x (Referents Entity)))
              (gerku $x)))))
    (Bind (($groups (Referents (Group Entity))
            (Refer
              (λ (($x (Referents (Group Entity))))
                (gunma $x $base)))))
      (Mention $groups))))
```

Because set and group object sorts are entity subtypes, `$sets` or `$groups`
can also fill a general `Referents<Entity>` predicate place by covariance. They
do not thereby unwrap to a raw mathematical `Set` or `Group` value.

## 11. Generalized quantifier witnesses

The `GQ`, its nuclear-scope function, and their application each have stable
identity. A witness is available only after that same application succeeds:

```lisp
(Smusni 0
  (Let (($gq (GQ Entity)
          (Exactly 3
            (λ (($x Entity))
              (gerku $x)))))
    (Let (($scope (Fn (Entity) Content)
            (λ (($x Entity))
              (bajra $x))))
      (Let (($run Content ($gq $scope)))
        (Do
          (Assert $run)
          (Bind (($dogs (Referents Entity)
                  (Witnesses $run)))
            (Assert
              (tatpi $dogs))))))))
```

This is not equivalent to counting members of `$dogs` after the fact. The full
quantifier function, scope function, and success identity remain explicit. A
request before successful execution uses local `(Referents Entity)` fallback
with reason `smusni.fallback.witness.before-success`, preserving the actual
reachable quantifier owner. `WitnessRequest` and `QuantifierApplication` are
not raw model object types and therefore never appear in its raw value.

## 12. Simultaneous termsets

When the graph explicitly licenses the coordinate-closed complete-product
profile, two generalized quantifiers with one polyadic nuclear scope reduce to
mutually constrained, coordinate-exhaustive sets:

```lisp
(Smusni 0
  (Assert
    (∃
      (λ (($dogs (Set Entity))
          ($people (Set Entity)))
        (∧
          (= (Card $dogs) 3)
          (= (Card $people) 2)
          (∀
            (λ (($dog Entity))
              (↔
                (∈ $dog $dogs)
                (∧
                  (gerku $dog)
                  (∀
                    (λ (($person Entity))
                      (→
                        (∈ $person $people)
                        (nelci $dog $person))))))))
          (∀
            (λ (($person Entity))
              (↔
                (∈ $person $people)
                (∧
                  (prenu $person)
                  (∀
                    (λ (($dog Entity))
                      (→
                        (∈ $dog $dogs)
                        (nelci $dog $person)))))))))))))
```

Nesting `$dogs` around `$people` or vice versa would impose an order that can
change generalized-quantifier truth conditions. The biconditionals also make
this stronger than arbitrary complete-product subset selection: if four dogs
all like the same two people, the displayed `Exactly 3`/`Exactly 2` content is
false. A graph which retains one coequal quantifier-bundle locus but lacks the
verified coordinate-closed profile uses local `Content` fallback with reason
`smusni.fallback.termset.profile-unverified`, preserving that complete raw
`Formula` object as in section 20. A graph already elaborated as an ordered
quantifier nest instead renders that ordinary nesting; the renderer does not
guess that it was formerly simultaneous.

## 13. Lambdas, abstractions, and event facets

A property abstraction needs only `λ`:

```lisp
(Smusni 0
  (Mention
    (λ (($x (Referents Entity)))
      (Close
        (melbi $x)))))
```

An event abstraction is a reference computation whose property shares the event
with all facets:

```lisp
(Smusni 0
  (Bind (($event (Referents Eventuality)
          (Refer
            (λ (($events (Referents Eventuality)))
              (∧
                (klama Speaker :Eventuality $events)
                (purci $events Now)
                (zvati $events Here))))))
    (Mention $event)))
```

The temporal and spatial facets are ordinary lowercase predicates of the same
event reference; no event-facet record or opaque PascalCase name is needed.

Reification is an explicit level crossing:

```lisp
(Smusni 0
  (Mention
    (Reify
      (klama Speaker))))
```

The inline predicate closes because `Reify` expects `Content`. An explicit
semantic scale fills the full-arity crossing:

```lisp
(Smusni 0
  (Mention
    (Measure
      (klama Speaker)
      DistanceScale)))
```

This last sample is normal output only when `DistanceScale` is a verified member
of the version-0 scale table; otherwise the affected scale value uses typed
fallback.

## 14. First-class acts and utterance facts

An act can be bound, targeted, and later performed:

```lisp
(Smusni 0
  (Let (($prior-act (Act Assertion)
          (Assert
            (melbi This))))
    (Let (($act (Act Assertion)
            (Assert
              (klama Speaker))))
      (Do
        (Express
          (Contrast Speaker $act $prior-act))
        $act))))
```

There is no `TargetFocus`; the target is statically `Act<Assertion>`.

An utterance with metadata and two co-realized acts keeps its token:

```lisp
(Smusni 0
  (Let (($prior-act (Act Assertion)
          (Assert
            (melbi This))))
    (Let (($current-act (Act Assertion)
            (Assert
              (klama Speaker))))
      (Utterance (($u UtteranceToken))
        (SpeakerOf $u Speaker)
        (AudienceOf $u Audience)
        (DeicticTimeOf $u Now)
        (DeicticPlaceOf $u Here)
        (Realizes $u $current-act)
        (Realizes $u
          (Express
            (Contrast Speaker $current-act $prior-act)))))))
```

The contrast's experiencer and two exact first-class act operands all remain
visible; its third role is the comparison target, not a generic utterance
anchor. Facts about `$u` are
analyzer facts, not extra speaker assertions; if an indicator instead targets
the utterance token, the same `$u` can be passed to its registered relation.

An actual relation that the speaker utters a reported transcript can itself be
asserted. The nested `Utterance` is a `TranscriptEntry` datum and is not
performed or contracted:

```lisp
(Smusni 0
  (Assert
    (Utters
      Speaker
      (Utterance (($reported UtteranceToken))
        (SpeakerOf $reported Speaker)
        (Realizes $reported
          (Assert
            (klama Speaker)))))))
```

That reports an utterance relation; it does not perform the nested transcript.

Address and directive force are first-class acts on the ordinary `Do` spine:

```lisp
(Smusni 0
  (Do
    (Vocative Audience)
    (Command Audience
      (klama Audience This))))
```

At a non-spine `Discourse` position, the level crossings are explicit. This
`Joi` connects two performances rather than merely storing their values:

```lisp
(Smusni 0
  (Let (($act (Act Assertion)
          (Assert
            (klama Speaker))))
    (Joi
      (Perform $act)
      (PerformUtterance
        (Utterance (($u UtteranceToken))
          (SpeakerOf $u Speaker)
          (Realizes $u
            (Assert
              (melbi This))))))))
```

Discourse transitions remain visible when the graph owns them:

```lisp
(Smusni 0
  (NewTopic
    (Perform
      (Assert
        (melbi This)))))
```

```lisp
(Smusni 0
  (Resume
    (Perform
      (Assert
        (klama Speaker)))))
```

## 15. Direct, embedded, and answered questions

A polar direct question:

```lisp
(Smusni 0
  (Ask
    (Polar
      (klama Speaker))))
```

An open argument question:

```lisp
(Smusni 0
  (Ask
    (OpenQ
      (λ (($x (Referents Entity)))
        (klama $x)))))
```

A relation question can bind an open-row predicate term when the answer's full
place structure is not yet known:

```lisp
(Smusni 0
  (Ask
    (OpenQ
      (λ (($relation
            (PredTerm
              (Row
                (1 (Referents Entity))
                (Eventuality (Referents Eventuality))
                Open))))
        (Close
          ($relation This))))))
```

A two-variable question retains an ordered heterogeneous answer tuple:

```lisp
(Smusni 0
  (Ask
    (OpenQ
      (λ (($who (Referents Entity))
          ($where (Referents Location)))
        (klama $who $where)))))
```

An indirect question is an inert object:

```lisp
(Smusni 0
  (Mention
    (QuestionOf
      (OpenQ
        (λ (($x (Referents Entity)))
          (cortu $x))))))
```

Positive polar answer content says which answer was selected:

```lisp
(Smusni 0
  (Assert
    (Answer
      (Polar
        (klama Speaker))
      (PolarAnswer Yes))))
```

An embedded `kau`-style answerhood commitment without a graph-recorded tuple
uses an honest contextual profile:

```lisp
(Smusni 0
  (Assert
    (Answer
      (OpenQ
        (λ (($x (Referents Entity)))
          (cortu $x)))
      (ContextualAnswer Exhaustive))))
```

It does not claim the selected `$x` is present in the graph.

When the graph does contain the ordered answer values, the tuple and
exhaustivity are explicit:

```lisp
(Smusni 0
  (Assert
    (Answer
      (OpenQ
        (λ (($who (Referents Entity))
            ($where (Referents Location)))
          (klama $who $where)))
      (TupleAnswer
        (Tuple Speaker Here)
        Exhaustive))))
```

An explicitly unresolved selection is a typed nullary value, not an invented
polarity or contextual answer:

```lisp
(Smusni 0
  (Assert
    (Answer
      (OpenQ
        (λ (($x (Referents Entity)))
          (cortu $x)))
      UnresolvedAnswer)))
```

## 16. De-re, de-dicto, nested, and opaque reference

An extensional graph-owned host can place a reference outside negation:

```lisp
(Smusni 0
  (Bind (($book (Referents Entity)
          (Refer
            (λ (($x (Referents Entity)))
              (cukta $x)))))
    (Assert
      (¬
        (nelci Speaker $book)))))
```

Inside an intensional desire place, a de-dicto reference remains local:

```lisp
(Smusni 0
  (Assert
    (djica Speaker
      (Bind (($wanted (Referents Eventuality)
              (Refer
                (λ (($events (Referents Eventuality)))
                  (Bind (($car (Referents Entity)
                          (Refer
                            (λ (($x (Referents Entity)))
                              (karce $x)))))
                    (pilno Speaker $car :Eventuality $events))))))
        $wanted))))
```

An explicit legal de-re owner may move `$car` outside that boundary:

```lisp
(Smusni 0
  (Bind (($car (Referents Entity)
          (Refer
            (λ (($x (Referents Entity)))
              (karce $x)))))
    (Assert
      (djica Speaker
        (Bind (($wanted (Referents Eventuality)
                (Refer
                  (λ (($events (Referents Eventuality)))
                    (pilno Speaker $car :Eventuality $events)))))
          $wanted)))))
```

A nested description remains inside the outer description property unless its
own owner says otherwise:

```lisp
(Smusni 0
  (Bind (($driver (Referents Entity)
          (Refer
            (λ (($x (Referents Entity)))
              (Bind (($car (Referents Entity)
                      (Refer
                        (λ (($y (Referents Entity)))
                          (karce $y)))))
                (pilno $x $car))))))
    (Assert
      (klama $driver))))
```

An `Opaque` lexical edge cannot be raised; a contradictory escaping owner is a
typed local fallback rather than an assumed extensional reading.

## 17. Respectively, collections, and math

Respectively is tuple/list construction plus typed `ZipWith`:

```lisp
(Smusni 0
  (Assert
    (ZipWith
      (λ (($speaker (Referents Entity))
          ($listener (Referents Entity)))
        (tavla $speaker $listener))
      (List Speaker Audience)
      (List Audience Speaker))))
```

Set and interval operations use conventional mathematical notation:

```lisp
(Smusni 0
  (Assert
    (∃
      (λ (($x Entity))
        (∈ $x
          (∩
            (SetOf
              (λ (($y Entity))
                (gerku $y)))
            (SetOf
              (λ (($y Entity))
                (blabi $y)))))))))
```

```lisp
(Smusni 0
  (Mention
    (Interval 0 1 Closed Open)))
```

An unsupported questioned math operator remains typed fallback; it is not
printed as an arbitrary PascalCase atom.

## 18. Signs and quotation

Opaque quotation preserves text without interpreting it:

```lisp
(Smusni 0
  (Mention
    (OpaqueQuote "mi klama")))
```

A structured quotation can mention a complete transcript entry without
performing it:

```lisp
(Smusni 0
  (Mention
    (StructuredQuote
      (Utterance (($u UtteranceToken))
        (SpeakerOf $u Speaker)
        (Realizes $u
          (Assert
            (klama Speaker)))))))
```

Raw name and sentence signs preserve their kind without minting a token:

```lisp
(Smusni 0
  (Mention
    (NameSign "alis")))
```

```lisp
(Smusni 0
  (Mention
    (SentenceSign
      (klama Speaker))))
```

A sign token can have text and denotation facts without conflating either with
interpretation:

```lisp
(Smusni 0
  (Mention
    (Sign (($s (SignToken Sentence)))
      (TextOf $s "mi klama")
      (Quotes $s
        (OpaqueQuote "mi klama"))
      (Label Item 1 $s)
      (Denotes $s
        (Reify
          (klama Speaker))))))
```

When interpretation is represented, the result family is explicit:

```lisp
(InterpretContent $sentence-sign)
(Let (($directive (Act Directive)
        (InterpretAct $performative-sign)))
  (Perform $directive))
```

Those two forms are fragments. The `Act<Directive>` annotation supplies the
force index which a standalone `InterpretAct` could not infer.

## 19. Recursion and identity

Mutually recursive inert functions use `LetRec`:

```lisp
(Smusni 0
  (LetRec
    (($even (Fn (Number) Content)
       (λ (($n Number))
         (∨
           (= $n 0)
           (∧
             (> $n 0)
             ($odd (− $n 1))))))
     ($odd (Fn (Number) Content)
       (λ (($n Number))
         (∧
           (> $n 0)
           ($even (− $n 1))))))
    (Assert
      ($even 4))))
```

The example demonstrates shape rather than a Lojban source sentence. Recursive
bindings whose initializers are not inert lambdas are not legal `LetRec` and
use fallback.

## 20. Fallback and diagnostic separation

A local unsupported value preserves its type, reason, fields, identity, and
sharing:

```lisp
(Smusni 0
  (Assert
    (=
      (Fallback Number "smusni.fallback.math.power-unregistered"
        (Object %1 "MathExpression"
          (Field "kind"
            (RawVariant "MathExpressionNodeKind" "Operator"
              (Field "operator"
                (RawVariant "MathOperator" "Power"))
              (Field "operands"
                (RawList
                  (Object %2 "MathExpression"
                    (Field "kind"
                      (RawVariant "MathExpressionNodeKind" "Literal"
                        (Field "literal"
                          (RawRecord "MathLiteral"
                            (Field "kind"
                              (RawTypedAtom "MathLiteralKind" "Integer"))
                            (Field "value"
                              (RawVariant "MathLiteralValue" "Integer"
                                (Field "value" (RawScalar "i64" "2"))))))
                        (Field "denotes" (RawNull))))
                    (Field "scalarNegation" (RawNull))
                    (Field "subscript" (RawNull))
                    (Field "common"
                      (RawRecord "SemanticObjectCommon"
                        (Field "source" (RawNull))
                        (Field "diagnostics" (RawList)))))
                  (Ref %2)))
              (Field "operatorDenotes" (RawNull))
              (Field "endpointInclusion" (RawNull))))
          (Field "scalarNegation" (RawNull))
          (Field "subscript" (RawNull))
          (Field "common"
            (RawRecord "SemanticObjectCommon"
              (Field "source" (RawNull))
              (Field "diagnostics" (RawList))))))
      4)))
```

If the renderer cannot establish a typed performable root, it preserves the
whole graph structurally:

```lisp
(Smusni 0
  (TypedGraph "SemanticGraph" "smusni.fallback.graph.root-not-performable"
    (Object %1 "SemanticGraph"
      (Field "version" (RawString "lojban-semantics-json-1"))
      (Field "root"
        (Object %2 "Parameter"
          (Field "sort" (RawVariant "SemanticSort" "Entity"))
          (Field "role"
            (RawTypedAtom "ParameterRole" "ArgumentQuestion"))
          (Field "introducedBy" (RawString "ma"))
          (Field "subscript" (RawNull))
          (Field "common"
            (RawRecord "SemanticObjectCommon"
              (Field "source" (RawNull))
              (Field "diagnostics" (RawList))))))
      (Field "objects"
        (RawMap
          (Entry
            (RawScalar "SemanticObjectId" "parameter:1")
            (Ref %2)))))))
```

Neither document contains a `Warning` node. The corresponding stable diagnostic
code and human message are written to stderr in the normal `gentufa` diagnostic
format.

## 21. Word cards

Requested dictionary cards occupy the optional third document slot:

```lisp
(Smusni 0
  (Assert
    (klama Speaker))
  (Words
    (Word klama "x1 goes to x2 from x3 via x4 using x5")))
```

The cards neither scope over nor modify the semantic body.
