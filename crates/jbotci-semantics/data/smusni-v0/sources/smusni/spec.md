# Human-readable smusni S-expression format

This document specifies version 0 of the human-readable `smusni` output format.
It is a typed, semantics-oriented projection of the tersmu graph. It is not a
transliteration of Lojban grammar and it is not a prettified XML tree.

The format has four goals:

1. preserve every semantic distinction represented by the input graph;
2. reduce grammar-shaped and record-shaped structures to a small compositional
   kernel;
3. remain readable to people familiar with conventional S-expressions,
   mathematical notation, and ordinary functional notation. This format is
   human-centric: where one reading is clearly predominant, the concise
   spelling expresses it implicitly and explicit operands are reserved for the
   uncommon committed case — complete coverage never requires exhaustive
   spelling;
4. be expressively complete for the semantic distinctions tersmu represents for
   successfully analyzed Lojban: each distinction has a typed composition of
   kernel primitives, lexical predicates, and reviewed transparent reductions.
   A renderer MUST NOT guess or silently omit a distinction. If an
   implementation cannot prove a faithful projection, it MUST return a
   projection failure and MUST NOT emit a smusni document.

Goals 1 and 4 are about the language; they are not claims about any particular
renderer. Three consequences follow:

- language expressiveness and implementation coverage are separate. A route may
  be fully specified here and not yet implemented;
- a failed implementation does not produce a conforming partial or raw
  document. There is exactly one kind of successful result, and a failure is
  not a document at all;
- a malformed or internally inconsistent semantic graph may fail independently
  of language expressiveness. Rejecting such a graph says nothing about whether
  smusni can express the corresponding distinction.

The keywords **MUST**, **MUST NOT**, **SHOULD**, and **MAY** are normative.
Examples are illustrative unless they are explicitly labelled as expansions.
The companion [samples](samples.md) exercise the rules in this document.
Section 16 defines the failure result; section 14.4 separates a coordinate's
normative semantic disposition from one implementation's coverage of it.

## 1. Semantic commitments

### 1.1 Predicate terms

The central relational value is a **predicate term**:

```text
PredTerm<ρ>
```

`ρ` is the ordered row of places that are still open. A lexical root is a
predicate term with its whole row open. Filling one place produces another
predicate term with a smaller row. A term with no numbered places left is still
a predicate term; saturation does not assert it and does not by itself close an
event variable or any other graph-owned default.

This deliberately treats the following as values of one family rather than as
different syntactic categories:

- a bare content predicate such as `klama`;
- `klama` with some arguments filled;
- `klama` with every numbered argument filled;
- a predicate term used under a modal, description, abstraction, or
  higher-order operator.

The word *predicate term* is used instead of *selbri*, *bridi*, *predication*,
or *proposition*. The first two are tied to Lojban surface grammar. The latter
two commonly suggest a completed or closed proposition, which is not required
here.

### 1.2 Functions

The other central value is a typed function:

```text
Fn<(A1 ... An), B>
```

Functions have ordered positional parameters. They are introduced by `λ` and
applied with ordinary S-expression application. Properties, generalized
quantifiers, open questions, relation-valued questions, event descriptions,
and eta-expanded place conversions are functions.

Predicate terms and functions are distinct because their application rules are
distinct. A predicate term has a named effective place row, supports labelled
fills and contextual closure, and retains lexical place provenance. A function
has only its declared ordered parameters and never acquires implicit predicate
places.

### 1.3 Closure, force, and performance

Four operations that Lojban surface syntax often leaves close together remain
distinct:

```text
fill a predicate place : PredTerm<ρ+p> × value -> PredTerm<ρ>
close predicate content: PredTerm<ρ> -> Content
construct an act       : Content -> Act<F>
perform an act         : Act<F> -> Discourse
```

`Content` is an evaluable, dynamically scoped proposition-like value. `Assert`
constructs an assertion act from content. It is never an implicit consequence
of filling a place. `Perform` is used only when the graph distinguishes an act
value from its performance. The `Smusni` document convention performs its
top-level `Performable`: an act or transcript entry directly on the implicit
performance spine, or a `Discourse` computation as written. The semantic
transcript crossing is `PerformUtterance`, but its wrapper is omitted on that
spine just as `Perform` is omitted for a top-level act.

Logical operators, quantifiers, presupposition, reference, and force are typed
higher-order intrinsics. Calling them “predicates” would not remove their
control or effect semantics. A dictionary predicate such as `zilvlina` cannot
replace logical disjunction unless the graph explicitly represents that
ordinary lexical predication.

### 1.4 Utterances and signs

Facts about an utterance or sign are ordinary predicate terms wherever
possible. Token creation, token identity, and actual performance are not
ordinary assertions and therefore remain explicit token boundaries.

Asserting that a person utters an act is not the same operation as performing
that act. The notation can represent both, but it MUST NOT conflate use with
mention.

## 2. Concrete syntax

### 2.1 Character and atom rules

Output is UTF-8 normalized to NFC. Whitespace separates tokens and is otherwise
insignificant. Consumers MAY accept semicolon line comments; the renderer MUST
NOT emit comments.

Strings use JSON escaping and double quotes. Integers are canonical decimal
without a leading plus sign or redundant leading zeroes. Exact non-integers use
`(/ numerator denominator)` with a positive denominator in lowest terms.

The namespaces are visually disjoint:

- lowercase symbols are lexical content roots: `klama`, `purci`;
- PascalCase symbols are registered primitives, transparent prelude names,
  types, or closed literals: `Assert`, `DescribedAs`, `Entity`, `Proximal`;
- conventional callable mathematical glyphs are registered primitives or
  transparent prelude functions: `∀`, `∧`, `≠`;
- `$name` is a lexically bound variable;
- `:2` and `:Eventuality` are literal place labels.

A bare symbol starts with a Unicode letter and continues with Unicode letters,
digits, apostrophe, hyphen, underscore, or period. A lexical spelling that is
not safe as a bare symbol uses conventional Lisp vertical-bar escaping:
`|...|`, with `\|` and `\\` escapes. Namespace markers are governed by their
own productions below; they are not arbitrary bare-symbol prefixes.

The complete callable-glyph set is:

```text
¬ ∧ ∨ → ↔ ⊕ ∀ ∃ = ≠ < ≤ > ≥ + − × ÷ ∈ ∪ ∩
```

`λ` is a special-form marker rather than a callable atom. No other punctuation
token is an atom. In particular, `/` is reserved for exact rational syntax.

PascalCase spellings are closed by this version of the specification. An
implementation MUST NOT mint an atom for an unknown or unavailable semantic
constructor. Projection fails instead, with a registered reason, and no
document is emitted.

### 2.2 Datum grammar

The grammar below describes canonical output. `datum` is recursively typed;
the grammar alone does not make an ill-typed application valid.

```text
document       ::= (Smusni 0 performable [words])
words          ::= (Words word-card+)

datum          ::= atom | string | integer | rational | variable
                 | application | special-form
atom           ::= bare-symbol | escaped-symbol | callable-glyph
callable-glyph ::= ¬ | ∧ | ∨ | → | ↔ | ⊕ | ∀ | ∃
                 | = | ≠ | < | ≤ | > | ≥ | + | − | × | ÷ | ∈ | ∪ | ∩
variable       ::= $ symbol-name
integer        ::= 0 | -? nonzero-digit digit*
positive-integer ::= nonzero-digit digit*
rational       ::= (/ integer positive-integer)
application    ::= (datum argument+)
argument       ::= datum | place-fill
special-form   ::= lambda | let | bind | let-rec | utterance | sign

place-fill     ::= :positive-integer datum
                 | :Eventuality datum
                 | (At datum datum)
place-label    ::= positive-integer | Eventuality

lambda         ::= (λ ((variable type)+) datum)
let            ::= (Let ((let-name type datum)) datum)
let-name       ::= variable | prelude-name
prelude-name   ::= PascalCase or glyph atom from the closed prelude registry
bind           ::= (Bind ((variable type datum)) datum)
let-rec        ::= (LetRec ((variable type datum)+) datum)

utterance      ::= (Utterance ((variable UtteranceToken)) utterance-item+)
sign           ::= (Sign ((variable (SignToken sign-kind))) sign-item+)
utterance-item ::= datum     ; statically Content
sign-item      ::= datum     ; statically Content
word-card      ::= (Word lexical-root string)

type           ::= type-atom
                 | (Referents type)
                 | (Set type) | (Group type) | (List type)
                 | (Interval type)
                 | (Tuple (type*))
                 | (Fn (type*) type)
                 | (PredTerm row)
                 | (RefComp type)
                 | (Act force)
                 | (Query (type*))
                 | (AnswerSelection (type*))
                 | (GQ type)
                 | (Sign sign-kind) | (SignToken sign-kind)
                 | (PlaceOf relation-ref type [(place-label+)])
row            ::= (Row row-slot* [Open])
row-slot       ::= (positive-integer type) | (Eventuality type)
relation-ref   ::= lexical-root | variable | relation-former
relation-former ::= (DropPlace relation-ref positive-integer)
                  | (Tanru relation-ref relation-ref)
                  | (Scalar scalar-kind relation-ref)
performable    ::= datum     ; statically Act, Discourse, or TranscriptEntry
force          ::= Assertion | Question | Directive | Expressive
                 | Mentioning | Address
sign-kind      ::= Name | Sentence | Quotation | Word | Letteral
                 | MathExpression | Connective | Text | Structured | Opaque
scalar-kind    ::= OtherThan | Opposite | Neutral
```

Square brackets in this grammar denote optional syntax and `*`/`+` denote zero
or more/one or more repetitions; they are not output characters.

`bare-symbol`, `escaped-symbol`, `symbol-name`, `type-atom`, and
`lexical-root` are lexical categories constrained by this section and the
closed registries in section 14. `digit` is an ASCII decimal digit and
`nonzero-digit` is `1` through `9`.

`performable` is an ordinary `datum` with the static requirement that its type
is `Act`, `Discourse`, or `TranscriptEntry`. There is no untyped top-level
alternative: a document body is always a typed value of the kernel language.

`/` in the `rational` production is a reserved grammar marker, not the callable
division glyph `÷`. The numerator may be negative; the denominator is positive,
and section 2.1 requires lowest terms. `lexical-root` in a word card is the
lowercase or escaped lowercase root whose displayed definition follows.

`Open` in `(Row slots... Open)` hides only an unknown tail of surviving
**numbered** places. An event-licensed relation must still list its
`(Eventuality (Referents Eventuality))` slot explicitly; `Open` never hides,
creates, or deletes the distinguished event place.

`λ` always prints a complete ordered typed parameter list. Placeholder lambdas,
implicit `$1` parameters, middle-dot holes, and bracket lambda sugar are not
version-0 syntax.

The lambda result type is inferred from its body. A body does not acquire
`Close` merely because it is inside `λ`; expected-type insertion applies only
when the surrounding operand or an enclosing declared `Fn` type requires
`Content`. Thus a lambda passed polymorphically to `Mention` can return a
`PredTerm`, while a lambda passed as a `Property<T>` closes its inline predicate
body. `Close` is explicit whenever no expected `Content` type is available.

`Let` and `Bind` each have exactly one binding in canonical output. Multiple
nonrecursive bindings are nested. This removes any ambiguity between parallel
and sequential binding. A `Let` initializer can be any inert typed value,
including a `Fn` introduced by `λ` or a predicate term. An explicit document
binding uses a `$variable`; a closed `prelude-name` occurs only in
the specification's implicit prelude described in section 14.1.

`LetRec` is the only multi-binding form. Its initializers are mutually visible,
and every initializer MUST be an inert `λ`; consequently every mutually
recursive reference is delayed inside a lambda body. Predicate terms,
`RefComp`s, acts, discourse, presuppositions, supplements, and other effectful
initializers are forbidden. `LetRec` denotes the recursive environment that
ties those function identities together. A consumer need not unroll or
normalize it, and applying a recursively bound function may diverge. Version 0
specifies no normal form for a recursive binding group whose initializers are
not all inert lambdas; that is a tracked spec gap under section 14.4, and a
graph which requires one makes projection fail with a registered reason.

### 2.3 Application

Ordinary application is type-directed:

- if the head is an `Fn`, the arguments fill its ordered parameters;
- if the head is a `PredTerm`, its arguments fill predicate places according to
  section 4;
- if the head is a registered primitive or prelude function, its registered
  signature applies.

Application is not implicitly left-associated. `($f a b)` is one application
of `$f` to two arguments. Curried application prints as `(($f a) b)` when its
types require two applications.

An empty list is not a value. Empty typed collections print `(Set Entity)` and
`(List Entity)`, where the first operand is the element type. Nonempty
collections omit the type only when the complete typed context uniquely
determines it. Otherwise the first child is the element type; type-position and
value-position namespaces make this unambiguous:

```lisp
(Set Entity)
(Set $x $y)
(Set Entity $x)
(List (Referents Entity))
(List $first $second)
```

### 2.4 Document packaging

Version 0 has exactly one top-level datum:

```lisp
(Smusni 0 performable)
```

If `--show-defs` is requested, an optional `Words` section is the third child:

```lisp
(Smusni 0
  performable
  (Words
    (Word klama "x1 goes to x2 from x3 ...")))
```

`Words` is reference data, not semantic content. Warnings and errors MUST NOT
appear in the datum; section 16 defines their separate channel.
If `--show-defs` resolves no definitions, the renderer omits the optional
`Words` section rather than emitting an empty one.

The document convention supplies one current utterance context with
speaker=`Speaker`, audience=`Audience`, time=`Now`, place=`Here`, deictic
ground=`CurrentGround`, and an actual generated locution event. A simple single
realized act may contract from `Utterance` to that act only on the implicit
performance spine, when its token is unreferenced and every omitted fact is
exactly one of these declared defaults. An `Utterance` used as an ordinary
datum operand never contracts because doing so would change its
`TranscriptEntry` type. Nondefault metadata, multiple acts, quotation, token
reference, or any additional fact also keeps the `Utterance` boundary. This is
`NotationDefault`, not provenance suppression.

A consumer that does not support version `0` MUST reject the document rather
than interpreting it as another version.

## 3. Types and level crossings

### 3.1 Core value families

```text
PredTerm<ρ>             predicate term with effective open row ρ
Fn<(A1 ... An), B>     ordered function
Content                closed dynamic proposition-like value
RefComp<T>             dynamic computation introducing a value T
Act<F>                 first-class act with force F
Discourse              performed act/discourse computation
TranscriptEntry        utterance token plus facts and realized acts
Performable            closed union Act<F>|Discourse|TranscriptEntry
Query<(A1 ... An)>     polar when the tuple is empty, open otherwise
Referents<T>           nonempty, number-neutral plural reference
Set<T>                 extensional mathematical set
Group<T>               ordinary group-object sort with component kind T
List<T>                ordered mathematical list
Tuple<(A1 ... An)>     heterogeneous ordered product
Interval<T>            ordered pair of endpoints with explicit inclusion
Sign<K>                sign value of kind K
UtteranceToken         graph-owned utterance identity
SignToken<K>           graph-owned sign identity
PlaceOf<R,T[,C]>       one of the compatible current places of relation R accepting T;
                       optional C narrows the candidate set explicitly
```

The closed primitive sort atoms used by version 0 are `Entity`, `Eventuality`,
`Achievement`, `Process`, `Activity`, `State`, `Experience`, `Locution`,
`Location`, `Amount`, `Scale`, `TruthValue`, `Epistemology`, `Concept`,
`AbstractNature`, `Proposition`, `Question`, `Text`, `Number`, `Natural`, and
`Cardinal`, plus the administrative `DeicticGround` sort. `Natural` is the
nonnegative-integer subtype of `Number`.
`Entity` is the top sort of ordinary first-order individuals. The primitive
subtype edges are explicit rather than inferred from this prose:

```text
Eventuality, Location, Amount, Scale, TruthValue, Epistemology,
Concept, AbstractNature, Proposition, Question, Text, Number,
Cardinal <: Entity

Achievement, Process, Activity, State, Experience, Locution
  <: Eventuality

Natural <: Number
```

There are no other primitive subtype edges. `Text <: Entity` denotes an
abstract text object. It does not erase a source quotation, sign, or sign token:
those values retain their `Sign<K>` or `SignToken<K>` boundary and cross it only
through the operations in section 3.3.

The constructed first-order object sorts `Set<T>`, `Group<T>`, `List<T>`,
`Tuple<(A1 ... An)>`, `Interval<T>`, `Sign<K>`, `SignToken<K>`, and
`UtteranceToken` are also subtypes of `Entity`. This is what lets a set, group,
sequence, mathematical value, or sign be talked about in an ordinary
entity-accepting predicate place. Higher-order and control families such as
`PredTerm`, `Fn`, `Content`, `RefComp`, `Act`, `Discourse`, `TranscriptEntry`,
`Query`, `AnswerSelection`, `GQ`, and `PlaceOf` are not made entities merely
because they are first-class typed values.

The only implicit sort conversions are one-way upcasts along these declared
edges plus the finite `Natural`-to-`Cardinal` embedding stated below.
`Referents` is covariant along them: if `A <: B`, then
`Referents<A> <: Referents<B>`. No corresponding downcast is implicit. Finite
`Natural` values have the canonical embedding into `Cardinal`. Other semantic
sorts require a registered versioned addition; until one exists, projection
fails with a registered reason and an unknown model sort is never printed as a
new type atom.

Consequently an eventuality may fill a general entity place, but a general
entity cannot fill an eventuality place. The subtype distinction remains
protective in the narrower direction.

Ordinary Lojban sumti places use `Referents<T>`, not raw `T`. A raw `T` MAY
singleton-lift wherever a registered operand requires `Referents<T>`, including
a lexical place. The reverse conversion is never implicit.

Integer literals greater than or equal to zero have principal type `Natural`;
negative integers and rational literals have principal type `Number`. A
nonnegative integer may upcast to `Number` or embed in `Cardinal` when the
expected type requires it. String literals have type `Text`.

### 3.2 Properties and generalized quantifiers

```text
Property<T> = Fn<(T), Content>
GQ<T>       = Fn<(Property<T>), Content>
```

A property is a function, not a special record. A generalized quantifier is a
higher-order value which accepts its nuclear-scope property. Restrictor,
importing behavior, and counting basis are retained in the function itself;
applying it yields ordinary `Content`, not a quantifier-record wrapper.

`PureProperty<T>` is specification metanotation for a `Property<T>` whose
application has `Γ = Δ`, an empty effect sequence, and a stable result at one
evaluation site for every argument. It is not a surface-printable type
constructor. A `PreludeRow` records this inferred refinement with the
registry-only schema form defined in section 14.2. Extensional comprehensions
and every reduction which duplicates a property require this stronger
judgment.

The refinement may occur recursively in a higher-order function signature.
For example, `Fn<(PureProperty<T>), Content>` is structurally the same surface
function family as `GQ<T>`, but its callable value retains the precondition that
the future nuclear-scope argument pass the purity checker. `Let` and other
identity-preserving bindings retain that refined callable signature.

`RefComp` denotes a dynamically hosted computation, not necessarily a context
mutation. A graph-`Fixed` bare `Context` lookup is read-only, has `Γ = Δ` and an
empty effect sequence, and is stable for its **lexical closure site** during one
force or discourse performance. Before beta expansion or sharing, every
graph-owned omitted place receives one closure-site identity. Repeated
applications of the same lambda reuse that identity and therefore resolve the
same fixed value within the performance; reperforming a stored act begins a new
performance and may resolve it again. Distinct omitted places receive distinct
identities even when they have the same type and dependencies.

An accommodation, reference introduction, or other update must be represented
by its actual effect and prevents purity until it is legally hoisted once.
Purity constraints are inferred checker refinements attached to function
values; `Let` and identity sharing preserve them even though `PureProperty`
does not print in a surface type annotation.

The refinement checker is structural and deterministic:

1. it assigns every lambda and lexical closure site a stable graph identity;
2. constants, variables, lexical predicate filling, fixed-context lookup,
   kernel primitives with normative summaries, and transparent prelude calls
   after expansion compose their context, effect, and stability summaries left
   to right;
3. `Let` copies the initializer summary to every use, while application
   instantiates the callee summary without minting new lexical closure sites;
4. `Bind`, `Refer`, accommodation, `Presuppose`, `Supplement`, act performance,
   and an unregistered or opaque call prevent `PureProperty` unless their effect
   has already been hoisted outside the candidate lambda; and
5. `LetRec` functions do not receive a purity refinement in version 0.

“Stable result” means that one lexical site, one argument tuple, and one
performance identify the same semantic value; it does not claim termination or
global rigidity across performances or possible worlds. If the above analysis
cannot establish the refinement, an extensional reduction requiring it is
unavailable and projection fails with a registered reason.

### 3.3 Explicit crossings

The following crossings are semantic operations, not harmless type coercions:

| From | To | Operation |
|---|---|---|
| `PredTerm<ρ>` | `Content` | `Close` |
| `Content` | `Act<Assertion>` | `Assert` |
| `Content` | `Proposition` | `Reify` |
| `Query<A>` | `Question` | `QuestionOf` |
| `Act` | `Discourse` | `Perform` |
| `TranscriptEntry` | `Discourse` | `PerformUtterance` |
| `T` | `Referents<T>` | `Singleton`, optionally elided at a statically known `Referents<T>` operand |
| `Sign<K>` or `SignToken<K>` | `Content` | `InterpretContent` |
| `Sign<K>` or `SignToken<K>` | `Act<F>` | `InterpretAct`, with `F` graph-known or expected |

`InterpretContent` and force-indexed `InterpretAct` are separate because a
static result type cannot be recovered from an undifferentiated
`Interpret : Sign -> Content|Act`. An act force which neither the graph nor the
expected position determines makes projection fail with a registered reason
rather than erasing the force.
`Reify` and `QuestionOf` construct one graph-owned proposition/question object
and therefore return a raw first-order value. Crossings whose graph result is a
number-neutral contextual selection return `Referents<...>` instead; this is a
result-cardinality distinction, not an implicit object/reference coercion.

### 3.4 Binding

`Let` names an inert value and preserves graph identity without evaluating an
effect. Its initializer is evaluated in the surrounding lexical environment.

`Bind` runs a `RefComp<T>`, binds its result, and evaluates its body in the
updated dynamic context. It is the only printed dynamic-value binder:

```lisp
(Bind (($x (Referents Entity) (Refer property)))
  ⟦body⟧)
```

Here and in later schema displays, `⟦...⟧` is specification metanotation and
not surface syntax.

The body may have any static result type `B`. The inferred effect judgment is:

```text
c    : RefComp<Γ,Δ;T;E1>
body : B<Δ,Θ;E2>
-------------------------
Bind x <- c in body : B<Γ,Θ;E1·E2>
```

The angle-bracketed contexts and effects are checker judgments, not surface
type syntax. Thus an argument-local `Bind` may return a reference value while
remaining dynamically inside that lexical argument. It is not interchangeable
with `Let`: the initializer computation runs at that operand's dynamic
evaluation site. When the body is an `Act`, the computation is stored with the
act and runs only when that act is performed; a binder may therefore
syntactically surround `Assert` while remaining semantically inside its force
segment.

An application evaluates effectful head and operand computations left to right
unless the registered operator gives control semantics of its own. Predicate
application only assembles its term; deferred operand computations run in that
order when `Close` evaluates the predicate content, subject to the lexical
place boundary rules in section 6.3.

Lexical scope and dynamic accessibility are distinct. A `$variable` is usable
only inside the body of its printed `λ`, `Let`, `Bind`, `LetRec`, `Utterance`,
or `Sign` binder. Dynamic context flow never makes an out-of-scope spelling
valid. If one graph identity is used at several sites, its binder's lexical body
MUST contain every use; if the graph instead represents a new contextual
resolution, that site receives its own `Bind`.

### 3.5 Contextual and deictic values

`Context` is a type-directed `RefComp<Referents<T>>`. Bare `Context` records a
fixed contextual computation. `(Context dependency...)` records an
underspecified computation which may depend on exactly those lexically bound
values, in graph order; it does not claim that the result actually varies with
them. It is used only where a contextual value is semantically present.

`Context` is not an anaphoric variable lookup by type. It does not mean “reuse
some preceding `Referents<T>`,” and it has no index for choosing among
same-typed values. A graph-resolved coreference prints the same bound variable;
a genuinely underspecified contextual choice prints `Context`; and a graph that
requires an identity but fails to supply one is rejected with a registered
invalid-graph reason.

The irreducible deictic constants are:

```text
Speaker  : Referents<Entity>
Audience : Referents<Entity>
Now      : Referents<Eventuality>
Here     : Referents<Location>
CurrentGround : DeicticGround
```

Entity deictics preserve proximity and ground through:

```text
Deictic : Proximity × DeicticGround -> Referents<Entity>
```

`Proximal`, `Medial`, and `Distal` are the closed proximity literals. The
readable defaults `This`, `That`, and `Yonder` are transparent prelude values
for the three applications to `CurrentGround`. The type reserves a graph-owned
noncurrent ground as a possible second operand, but version 0 has no normal
binder or constructor for one. That is a tracked spec gap under section 14.4:
every currently encountered noncurrent ground makes projection fail with a
registered reason. Thus the convenient constant does not collapse source `ti`,
`ta`, and `tu` or discard their ground.

## 4. Predicate rows and place filling

### 4.1 Effective rows

Every predicate term carries an **effective row**. Its public numbered labels
belong to the relation expression visible at that point. They are not claimed
to be universal semantic roles.

Internally, a surviving lexical slot also carries provenance
`(normalized lexical root, original dictionary ordinal)`. That provenance is
used only for dictionary metadata, arity validation, and lexical scope-policy
lookup. It is a root-relative coordinate, not a human-visible semantic name.

### 4.2 Fill cursor

For a lexical predicate application, the numbered cursor starts at the first
current numbered place.

1. A plain operand fills the cursor place and advances to the next surviving
   current place.
2. `:n value` fills current place `n` and moves the cursor to the first
   surviving unfilled place after `n`. Earlier skipped places remain open.
3. `:Eventuality value` fills the distinguished event place and does not move
   the numbered cursor.
4. `(At place value)` fills a computed current place and makes the numbered
   cursor statically unknown. Every plain or literal labelled fill MUST precede
   the first computed fill in canonical output; only further computed fills may
   follow it.

Thus:

```lisp
(pilno :2 $car $event)
```

fills current x2 with `$car`, then fills current x3 with `$event`; x1 remains
open for `Close`.

Duplicate fills, a nonexistent place, an incompatible value, a second event
fill, or a fill of a deleted place is a projection error. A computed place is
typed `PlaceOf<R,T>` or `PlaceOf<R,T,C>`. `R` is the current relation expression
or a bound relation identity, not merely its lexical root. Before any computed
fill is evaluated, the whole application reserves every place selected by its
preceding plain and literal fills.

The omitted candidate set denotes exactly all surviving current places of `R`
that remain unreserved at the binding's `At` host and accept `T`; this
derivable set contains numbered places only. Canonical output omits the list
when the graph's question domain is that derivable set. It prints `C` only when
the graph narrows the domain further, or when a shared place variable has
multiple hosts from which one domain cannot be recovered. `Eventuality` may
occur in an explicit `C` only when the graph's question domain genuinely
includes the distinguished event place. Every explicit set MUST exclude
reserved or deleted places and every place that rejects `T`. Candidate sets for
multiple computed fills MUST be pairwise disjoint; an overlapping or otherwise
noninjective assignment is rejected with a registered invalid-graph reason.

### 4.3 `At`

`At` exists only for a genuinely computed place, principally a graph-preserved
`fi'a` question or variable:

```lisp
(OpenQ
  (λ (($p (PlaceOf klama (Referents Entity))))
    (klama (At $p This))))
```

Literal numeric positions use `:n`, not `(At 2 value)`. Predicate-term-valued
modal designators do not exist. Modals normalize through `Joi` as specified in
section 10.

`At` is a typed surface form for dependent row application, not an independent
semantic relation. Given a finite candidate row, it could be expanded to a
case split that compares the place value and performs the corresponding
literal fill in each branch. Canonical output retains `At` because that generic
expansion would duplicate the surrounding predicate body and obscure the
question being asked. Consequently a document with no first-class place value
has no `At`; sparse and out-of-order literal fills use `:n` alone.

An open `mo`-like relation question may bind a predicate term whose total row
is not yet known. `(Row slots... Open)` records every statically required slot
and an unknown surviving tail. Applying the variable fills only known current
slots. Answer substitution supplies the concrete relation and its remaining
row before the deferred `Close` is checked; only answers whose resulting row
is closeable inhabit the query. This is not a row-erased escape hatch:
candidate relations and all represented slot constraints remain part of the
query graph, and an incompatible answer is rejected with a registered
invalid-graph reason.

### 4.4 `zi'o`

`DropPlace` represents semantic deletion of a numbered place:

```text
DropPlace : (r : PredTerm<ρ>) × (positive integer literal p in ρ)
            -> PredTerm<ρ-p>
```

Surviving current labels do not renumber after deletion; the deleted label is a
visible hole and plain traversal skips it. `DropPlace` cannot delete the
distinguished event place. Its second operand is never a computed `PlaceOf`
value. In nested deletions each literal is checked against the current row
produced by the preceding deletion, whose surviving numeric labels remain
stable.

Deletion requires positive evidence that the derived relation lacks that
semantic role. A missing source operand is not such evidence: ordinary omission
continues to use `Close` and its distinct contextual value. `DropPlace` is not
a concise spelling of `zo'e`.

Positive evidence may be an explicit graph relation whose declared role set
omits the lexical role, or a versioned reduction contract which defines a
derived relation as lacking it. An explicit source `zi'o` supplies direct
positive evidence for its own deletion. Every shipped generated deletion
records that semantic absence and an evidence id in the checked-in registry.
Surface elision, an absent model field, or failure to find a filler is never
evidence.

### 4.5 Place conversion

Source `se`, `te`, `ve`, and `xe` are consumed during semantic elaboration.
Ordinary output prints the base lexical predicate with values mapped to the
appropriate base-root places. No `Se`, `Te`, `Ve`, or `Xe` intrinsic exists.

If a converted relation itself escapes as a first-class value, it is
eta-expanded as an ordered function. For a binary relation:

```lisp
(λ (($x A) ($y B))
  (root $y $x))
```

The receiving place must expect the corresponding `Fn` type. A graph that
requires an escaped converted `PredTerm` with labelled-place behavior, rather
than an ordered function, cannot be represented by pretending the lambda is a
predicate term. Version 0 specifies no other route for it; that is a tracked
spec gap under section 14.4, and projection fails with a registered reason.

### 4.6 Relation formers

`Tanru`, `Scalar`, and `DropPlace` are registered relation formers. Each
declares its result row and a total mapping from every surviving surface slot to
lexical provenance. A former with unknown row behavior cannot print in normal
form.

`Tanru modifier head` preserves the head's effective row and records only the
semantically represented vague modification. It MUST NOT invent a specific
dictionary predicate.

The version-0 scalar form is:

```text
Scalar : ScalarKind × PredTerm<ρ> -> PredTerm<ρ>
```

`OtherThan`, `Opposite`, and `Neutral` are the closed `ScalarKind` literals.
The model's `Affirmed` case is the identity transformation and is a
`ProvenDesugaring`, so it does not print. A scalar operation with an
independently represented scale or restricted argument-place scope requires a
separately registered exact reduction; until that row exists, projection fails
with a registered reason.
Displayed-content intensity and phase likewise lower through registered
ordinary predicates or transparent prelude helpers, not generic `Degree` or
`Phase` constructors.

## 5. Closing predicate content

### 5.1 Semantics of `Close`

`Close` converts a closeable predicate term to `Content`. It performs all and
only the following graph-licensed closure steps:

1. every remaining defaultable ordinary referential place whose graph
   dependence is `Fixed` receives a fresh bare `Context` computation;
2. graph-shared defaults remain shared through an explicit `Let` or `Bind`;
3. an open distinguished event place receives a local existential event when
   the root/event former licenses one;
4. existing event facets and references determine the event variable's scope;
5. no higher-order, function, content, sign, or act place receives an invented
   default.

Each silent ordinary default is distinct unless explicit graph identity says
otherwise. “Fresh” here means a distinct lookup identity, not a context update.
A graph-`Fixed` lookup is the read-only, site-stable computation specified in
section 3.2, so it may occur inside a `PureProperty`. An
`Underspecified { mayDependOn }` default cannot be hidden by
`Close`: it is bound explicitly from `(Context dependencies...)` and the same
bound value fills the place. This preserves the exact permitted dependency set
rather than replacing it with “all accessible binders.”

`Fixed` means independent of enclosing semantic binders, not uniquely known by
the speaker or rigid across possible interpretations. A bare `Context` may
resolve any graph-permitted contextual value at that site. A genuinely
quantified or reference-introducing reading remains an explicit quantifier or
`Refer`; `Context` does not silently manufacture one.

The omitted `Context` computations run left to right in current numbered-place
order at the dynamic evaluation site of `Close`, inside the same content and
eventual act performance, before the lexical predication is evaluated. Each is
local to that closure unless graph identity has caused it to be bound and
shared explicitly.

If any remaining place is not registered as defaultable, or event ownership is
ambiguous, `Close` is not defined for that row and projection fails with a
registered reason. Saturation alone never licenses closure.

### 5.2 When `Close` may be omitted

Canonical output omits `Close` exactly when all of these conditions hold:

- the expression is syntactically inline and not referenced elsewhere;
- its effective row is statically known, or every branch of a finite computed
  fill has been checked to leave a row closeable by the same standard rules;
- the surrounding registered operand position requires `Content`;
- closure uses only the standard defaults above;
- no default or event identity must be named outside the expression.

Consequently this is canonical:

```lisp
(Assert (klama Speaker))
```

and elaborates as if the inline predicate term were wrapped in `Close`.

A bound, shared, nonstandard, or independently targeted predicate
term prints `Close`:

```lisp
(Let (($p (PredTerm
             (Row
               (Eventuality (Referents Eventuality))))
        (klama Speaker This Speaker This This)))
  (Assert (Close $p)))
```

Expected-type insertion is available at every registered `Content` operand,
including logical operators, `Reify`, `Polar`, and abstraction operators. It is
never available merely because the expression appears at top level.

This subsection governs canonical document output. Specification-level
elaborated core may spell an otherwise insertable `Close` when explaining an
equation, but the transparent prelude definitions in section 14.1 use the same
canonical omission as documents. Prelude equivalence is checked after
expected-type insertion has made every omitted crossing explicit.

## 6. Dynamic content and accessibility

### 6.1 Context model

The notation does not print a generic `State` or `DiscourseMonad` wrapper.
Nevertheless, every static result family has an inferred dynamic judgment;
the important specializations are:

```text
Content<Γ,Δ;E>
RefComp<Γ,Δ;T;E>
A<Γ,Δ;E>
```

`Γ` is the input discourse context, `Δ` the successful output context, and `E`
the ordered side effects. This typing is usually inferred and omitted from
surface binder annotations.

The dynamic interpretation is what gives anaphora the correct accessibility
across connectives. `Assert` does not create this behavior: it constructs an
act which stores content whose accessibility behavior is already defined.
Crossing that act through the implicit top-level performance spine or explicit
`Perform` executes the stored content.

A **force segment** is one dynamic execution of the content stored by a single
`Act<F>`, beginning when the implicit performance spine or `Perform` crosses
that act and ending at its corresponding force handler. Merely constructing an
act does not begin a segment. A nested act is inert until separately performed;
that performance begins a distinct segment.

The checker represents a context as graph-identity capabilities annotated with
their lexical/multiplicity region, and represents `E` as an ordered sequence of
typed effect tokens. It computes judgments bottom-up:

- a pure atom or inert value has `Γ -> Γ` and no effects;
- ordinary application composes the registered head and operand judgments left
  to right;
- conjunction and content `Joi` compose operand outputs in order;
- alternatives start from the same input and export only identities explicitly
  bound outside every branch;
- negation and quantifier bodies export no ordinary body-local capability;
- an implication supplies the antecedent's successful output to its consequent
  but exports neither operand's ordinary new capability afterward;
- act construction stores its content judgment without executing it, while a
  spine item, `Perform`, or `PerformUtterance` instantiates the stored judgment
  at that performance site; and
- each control/effect primitive applies its exact row below rather than ordinary
  application composition.

The result of a branch join is therefore determined by graph identity and
region equality, not by matching types or spellings. Any unregistered control
form or unresolved join fails at the smallest typed owner. Together with the
purity algorithm in section 3.2, the placement and handler rules in sections
6.3 and 6.4, the witness-flow algorithm in section 9.4, and registry validation
in section 14.2, these rules are the complete version-0 checker algorithm; an
implementation may choose different internal data structures but not different
transfer behavior.

### 6.2 Accessibility table

The following rules are normative. “Exports” means available to later dynamic
resolution in a successful continuation; it never extends lexical variable
scope. Whenever a later term uses the same graph identity, one printed binder
must already enclose both sites.

| Form | Evaluation and accessibility |
|---|---|
| `(∧ a b ...)` | left to right; each successful operand sees and exports the preceding context |
| `(Joi a b ...)` | left to right at the recorded nonlogical locus; same context flow as conjunction, but retains nonlogical connection identity |
| `(∨ a b ...)` | branches share the same input; branch-local ordinary introductions do not escape; a graph-owned identity valid in every branch is bound outside the disjunction and used explicitly in each branch; handled projective effects follow section 6.4 |
| `(¬ a)` | `a` sees the input; ordinary at-issue/reference introductions inside it do not escape, while handled projective effects follow section 6.4 |
| `(→ a b)` | `b` sees the successful dynamic context of `a`; a referent used in both operands is nevertheless printed under one lexical binder spanning the whole conditional; neither operand's new ordinary introductions escape by default; handled projective effects follow section 6.4 |
| `(↔ a b)` | has classical biconditional truth conditions and is primitive rather than a duplicating rewrite; `a` and `b` each see the same input context, each printed occurrence is evaluated at most once per performance, and neither operand's ordinary introductions flow into the other or escape; a shared identity is bound outside and handled projective effects follow section 6.4 |
| `(⊕ a b)` | alternatives share input; branch-local ordinary identities do not escape; handled projective effects follow section 6.4 |
| `(∀ (λ (...) body))` | lambda variables and ordinary body introductions are local; handled projective effects follow section 6.4 |
| `(∃ (λ (...) body))` | lambda variables and ordinary body introductions are local; handled projective effects follow section 6.4; version 0 has no plain-existential witness export, so later anaphora must use a graph-licensed `Refer`/generalized-quantifier route, and otherwise projection fails with a registered reason rather than using the variable out of scope |
| generalized quantifier | variable is local; only its registered successful run handle can authorize `Witnesses` |
| `(Presuppose trigger body)` | emit the projective `trigger` at its dependency-legal handler, resolve or accommodate it once there, then evaluate `body`; the trigger's successful context is visible in `body` and in the handler continuation, while any shared lexical identity is bound outside the whole form |
| `(Supplement body side)` | evaluate `body` in situ and emit `side` once as an ordered supplement-family effect at its graph-owned legal handler; `side` is evaluated in the handler's context at its commitment point, ordered after the handler's at-issue content containing `body`; intervening shells contribute only what their own rows export, so `side` never sees body-local introductions across a `¬`, `∨`, `→`, `↔`, `⊕`, quantifier, or query shell; `side` is not an operand of a truth-functional or force operator around `body`, and exports only graph-declared shared identities; section 6.4 defines projection and commitment through the listed transparent shells |
| `(Bind ... body)` | run the computation, bind its result, then evaluate the body; export follows the body's host rule |
| `(Let ... body)`, `(LetRec ... body)` | bind inert identity without changing dynamic context; only `body` is evaluated at the enclosing dynamic host |
| `Refer`, `Context` | run at their printed or expected-type-inserted operand host and introduce only their declared result/effects |
| `(Witnesses run)` | retrieves the graph-exported selection of one uniquely dominating successful quantifier run; it does not rerun the quantifier |
| `(Reify content)` | closes a proposition object; ordinary introductions do not escape the reified content |
| `Answer`, `QuestionOf` | query-local variables do not escape the crossing |
| lexical `Intensional` place | traps ordinary reference raising at that argument boundary |
| lexical `Opaque` place | traps all reference raising and de-re export |
| `Assert`, `Ask`, `Command`, `Express` | construct acts; context export occurs when the act is performed |
| `Perform`, `PerformUtterance` | explicitly cross a stored act or transcript entry to discourse and run its deferred dynamic content |
| `(Do a b ...)` | performs its `Act`, `Discourse`, or `TranscriptEntry` operands directly from left to right; bare acts and transcript entries use the implicit performance spine, and each successful item sees the context exported by earlier items |
| `Utterance`, `Sign` | bind a fresh token only inside their analyzer facts; constructing the returned transcript/sign value exports no discourse reference by itself |
| `NewTopic`, `Resume` | preserve their graph-recorded discourse transition while otherwise forwarding the enclosed discourse's ordered effects |

No connective is permitted to inherit an unspecified “usual” accessibility
rule.

### 6.3 Dynamic reference placement

Every dynamic reference edge has:

- its lexical evaluation site;
- the free-variable dependencies of its property;
- a lexical place policy, when it is an argument of a lexical root;
- an optional graph-owned de-re host.

The closed lexical policies are:

- `Extensional`: the reference may be hosted outside the lexical argument;
- `Intensional`: it remains inside that lexical argument unless an explicit
  legal de-re host is present;
- `Opaque`: it remains inside and rejects an escaping de-re host.

Surface current coordinates map through row provenance to the lookup key
`(root, original ordinal)`. Scope policy is a property of the semantic place,
not of the particular reference or other dynamic computation encountered
there. Missing, contradictory, or unattested policy metadata fails closed;
spelling, argument type, nearby rows, and converted surface position are never
heuristics.

If the graph supplies a de-re or de-dicto host, that exact host is used after
legality and dependency checks. Otherwise an ordinary xorlo `Refer` in an
`Extensional` position denotes a fixed reference within its force segment. Its
`Bind` raises over transparent administrative shells and visible extensional
`¬`, `∨`, `→`, `↔`, and `⊕` to the outermost legal point inside its semantic
force segment, reification, lambda-dependency, `Intensional`, or `Opaque`
boundary. That boundary is semantic rather than a syntactic-parent test: the
canonical binder may enclose an act constructor so its variable spans the act,
while the computation remains deferred until the act is performed. `Joi` and
`Close` are transparent for placement within the selected host.

A reference that depends on an enclosing variable remains inside that
variable's lexical scope, and the dependency appears in the printed binder or
lambda; the renderer never freezes a “may depend” edge into a constant by
omission. An explicitly local reference remains at its graph owner. `Context`
stays at its represented evaluation site. If these rules and the graph do not
determine one legal host, the smallest affected dynamic subgraph is rejected
with a registered invalid-graph reason. Source order orders effects at one
already determined host; it never chooses semantic scope.

### 6.4 Side-effect handlers

`Presuppose`, `Supplement`, expressive acts, and utterance facts are separate
effect families. A construct handles only its declared family. Projective
presuppositions are handled by the nearest performed force segment, an
enclosing `Do`/discourse sequence, or the document performance convention,
subject to the dependency and boundary rules below. The nearest enclosing
legal handler wins; equal-depth candidates are ordered by source order. Effects
generated inside a supplement do not become at-issue content. Effects generated
inside opaque or reified content cannot be hosted outside that boundary.

Only a performance which evaluates stored content is a force-segment handler:
one with force `Assertion`, `Question`, `Directive`, or `Expressive`.
`Mention` and `Vocative` operands are inert values; performing those acts
instantiates no operand-content judgment and commits no presupposition,
supplement, or reference effect contained in that value. `Utterance` and `Sign`
analyzer facts likewise do not become force handlers merely because their token
is constructed. A projective effect whose search reaches a `Mention`,
`Vocative`, utterance/sign analyzer-fact boundary, `Reify`, or `Opaque` boundary
without finding a legal handler inside it has an absent handler, and the
smallest affected content is rejected with a registered invalid-graph reason.

`Presuppose` is explicitly projective rather than an ordinary left-to-right
conjunction. Its trigger may be handled outside transparent `¬`, `∨`, `→`, `↔`,
`⊕`, and quantifier shells when all of its free dependencies remain in scope.
At that handler it is resolved or accommodated exactly once per performance;
the resulting context is supplied to the printed body and to the handler's
successful continuation. This is the stated exception to the ordinary
branch/negation export rows above and is why `Every` retains its nonempty
restrictor commitment under outer negation. A dependency boundary, force
boundary, `Opaque` place, or reification blocks further projection. If branch
filtering or competing possible handlers are represented but do not determine
one exact placement, the smallest affected content is rejected with a registered
invalid-graph reason rather than applying a generic natural-language projection
heuristic.

`Supplement` is likewise projective, but carries a separate assertional
supplement effect rather than a presupposition trigger. Its graph-owned anchor
either names an explicit dependency-legal local handler or uses the nearest
performed force segment, enclosing `Do`/discourse sequence, or document
performance convention.
The handler commits `side` exactly once per performance, ordered after the
handler's at-issue content containing `body` has been evaluated. The at-issue
value of `(Supplement body side)` is the value of `body`; commitment to `side`
is not part of `body`'s at-issue truth conditions.
Consequently it is not negated, questioned, made conditional, or made
alternative merely because the printed `Supplement` occurs inside transparent
`¬`, `∨`, `→`, `↔`, `⊕`, generalized-quantifier, or query shells. It projects
through those shells to its recorded handler exactly when all dependencies
remain in scope. A dependency boundary, force boundary, `Opaque` place, or
reification blocks further projection. A graph which records a genuinely
local or conditional side contribution must name that distinct legal handler;
the renderer never infers local versus projective interpretation from surface
nesting. Once the handler is fixed, canonical output raises `Supplement` across
every transparent shell to the outermost type-correct position inside that
handler and all dependency boundaries. For an assertion this is immediately
inside the act's `Assert`; for a polar question it cannot cross the
`Content -> Query` boundary and is therefore immediately inside `Polar`.
Canonical output never retains a lower equivalent attachment merely because it
matches source nesting. The printed placement must make the selected handler
the nearest legal handler under these rules; a graph anchor naming a different
handler is illegal because the output would not preserve it. An absent,
illegal, or ambiguous supplement handler makes the smallest affected content a
registered invalid-graph rejection. Nested effects generated while evaluating
`side` retain
their own families and handler rules, and do not become at-issue content by
being inside a supplement.
A supplement whose side depends on a variable or dynamic introduction that is
not available in any legal version-0 handler cannot be hoisted out of that
scope. Version 0 has no hidden dependent-supplement handler; that is a tracked
spec gap under section 14.4, and projection fails with a registered reason.

Reference resolution may have a legal `Discourse` host, including a graph-owned
`Bind` spanning several acts in `Do`. The discourse handler runs that
computation before the first dependent performance and exports its successful
context to the sequence. Its descriptive property is a reference-resolution
condition, not an `Assert` act and not an extra speaker assertion. When no
explicit utterance token survives, the top-level document convention provides
the act/transcript-performance and discourse-reference handlers, but it does
not invent utterance identity or metadata.

## 7. Acts, discourse, utterances, and signs

### 7.1 Acts are first-class values

The force constructors are:

```text
Assert  : Content -> Act<Assertion>
Ask     : Query<A> -> Act<Question>
Command : Referents<Entity> × Content -> Act<Directive>
Express : Content -> Act<Expressive>
Mention : A -> Act<Mentioning>
Vocative: Referents<Entity> -> Act<Address>
```

The input model's utterance-force cases are dispositions, not an invitation to
mint more force constructors:

| Model force | Normal form |
|---|---|
| `Assert` | `Assert content` |
| `Ask` | `Ask query` |
| `Command` | `Command addressee content`; the addressee is the graph-resolved command target, or the utterance audience only when that equality is a declared notation default |
| `Mention` | `Mention value` |
| `Vocative` | `Vocative addressee` plus any separately represented content |
| `Quote` | `Mention sign` when the graph supplies that sign identity; otherwise projection fails with a registered reason |
| `Parenthetical` | `Supplement host side` at the graph's aside owner; it has no standalone `Parenthetical` act |
| `Subordinated` | inert content under its owning connective, query, abstraction, or act; it has no standalone `Subordinated` act |

If a parenthetical or subordinated node lacks the owner needed by these
reductions, the smallest such utterance is rejected with a registered
invalid-graph reason. `Express` is the
act constructor for separately represented displayed/expressive content; it is
not an extra `UtteranceForce` spelling.

The content or target of any act may be bound by `Let` and referred to by
ordinary predicates. No `TargetFocus` enum is used; the target's identity and
static type say whether it is a clause, predicate term, act, sign, event, or
other value.

`Do` sequences performables:

```text
Do : Performable^n -> Discourse, n >= 2
```

It evaluates left to right and preserves graph/source order. A one-item sequence
has no distinct version-0 semantics and contracts to that item; at an ordinary
`Discourse` operand an `Act` uses `Perform`, a `TranscriptEntry` uses
`PerformUtterance`, and an existing `Discourse` remains as written. `Perform act`
performs an act only when a position statically requires `Discourse` away from
the implicit performance spine. An `Act` or `TranscriptEntry` operand of `Do`,
or the top-level `Act` or `TranscriptEntry` of `Smusni`, prints directly and
MUST NOT be wrapped in `Perform` or `PerformUtterance`; the enclosing spine
performs it. `PerformUtterance entry` performs the acts realized by a transcript
entry when an ordinary non-spine position explicitly requires `Discourse`.

An act value is a reusable deferred computation, not a unique performance
token. If the graph applies the same act value at two distinct performance
sites, its deferred dynamic content runs once at each site in source order.
Graph identity preserves the act value; it does not collapse two represented
performances into one.

`NewTopic discourse` and `Resume discourse` remain only when the semantic graph
records the corresponding paragraph transition. Mere source paragraph layout
is provenance and does not print.

### 7.2 Utterance-token boundary

`Utterance` is a fresh-token binder and transcript constructor, not `Let` sugar:

```lisp
(Utterance (($u UtteranceToken))
  (SpeakerOf $u Speaker)
  (Realizes $u (Assert content)))
```

Every item is `Content`, normally an inline predicate term closed at the item
boundary. It is an analyzer fact about the same `$u`, not a performed speaker
assertion. `Realizes` is one such ordinary predicate over the token and a
first-class act. Multiple `Realizes` facts identify co-realized acts in that
order; they are not silently conjoined into one assertion.

Registered utterance facts include `SpeakerOf`, `AudienceOf`, `LocutionOf`,
`DeicticTimeOf`, `DeicticPlaceOf`, `Realizes`, and registered relations such as
`Utters` when the graph actually contains them. A fact is analyzer
content about the token, not automatically a speaker assertion.

`LocutionOf` relates the token to a `Locution` eventuality. Surface wording is
represented separately by `TextOf`, and a graph-owned sign of that wording is
represented by a `SignToken` plus its facts. `StructuredQuote` of the complete
transcript is therefore distinct from quotation of that wording sign.

`Utterance` returns `TranscriptEntry`. The boundary MUST remain when the token
identity is referenced or quoted, any nondefault metadata survives, more than
one act is realized, or the entry occurs in any ordinary datum position,
including a `PerformUtterance` or `StructuredQuote` operand. Only on the
implicit performance spine may a simple entry whose sole identity-dependent
fact is one `Realizes` fact and whose omitted metadata is exactly the document
defaults contract to that act.

### 7.3 Sign-token boundary

`Sign` similarly binds a fresh sign token:

```lisp
(Sign (($s (SignToken Sentence)))
  (TextOf $s "mi klama")
  (Denotes $s (Reify (klama Speaker))))
```

The closed sign kinds are:

```text
Name Sentence Quotation Word Letteral MathExpression Connective Text
Structured Opaque
```

The raw sign constructors are `NameSign`, `SentenceSign`, `StructuredQuote`,
and `OpaqueQuote`. A sign token and a raw `Sign<K>` are distinct. Either can
singleton-lift to `Referents<Sign<K>>` or `Referents<SignToken<K>>` at a
statically known referential operand; the resulting reference may then upcast
along the ordinary first-order subtype edges, including to
`Referents<Entity>`. An intrinsic which expects the raw sign or token receives
it without lifting, and no upcast erases which sign boundary the graph owns.
Every `Sign` item is analyzer `Content` about the pre-bound token; the whole form
returns that `SignToken<K>` with its associated facts.

`InterpretContent` and `InterpretAct` are used only when interpretation itself
is represented. Quotation, denotation, and token facts do not imply
interpretation.

### 7.4 Indicators and displayed content

An indicator's represented participants are first-class values. Its primary
target is the actual `Content`, `PredTerm`, `Act`, event, sign, or token that the
semantic graph identifies. Clause-versus-predicate focus follows from that
identity and type; no `TargetFocus` value prints.

The displayed-content family selects a registered relation table; the exact
relation then prints as a predicate. There is no universal “anchor” place:
each relation row names and types all of its roles. For example, version-0
`Contrast` is one type-parameterized generated relation with the signature
`Referents<Entity> × T × T -> Content`: it has experiencer, contrasted target,
and comparison target, so both latter operands may be first-class acts at the
same monomorphic `T`, as in `samples.md` section 14. Different target types
require a separately registered exact relation; until that row exists,
projection fails with a registered reason rather than unifying unrelated
values. A different indicator may instead take an utterance
token or another graph-owned anchor. Polarity, intensity, phase, and modifiers
lower through registered relation formers or additional predicates.
`TargetFocus` is recoverable from target identity/type and is a proven
desugaring, not discarded provenance. If any remaining field has no
compositional lowering, projection fails with a registered reason rather than
retaining a generic indicator record.

The graph's assertion effect controls act construction:

| Effect | Normal-form consequence |
|---|---|
| `None` | preserve the host act and add only the represented expressive relation |
| `HostAsserted` | the host content is an `Assert` act at this utterance boundary |
| `HostSubordinated` | retain the host content as a first-class target but do not construct an independent host assertion |
| `MetalinguisticallyVoided` | do not assert the displayed host content; perform only the metalinguistic/expressive act and any other graph-recorded acts |
| `Performative` | construct and perform the graph-identified performative act rather than treating its wording as a truth-conditional assertion |

Multiple acts use `Do` or ordered `Realizes` facts. Nothing analogous to
`AssertionEffect` prints after the act structure has been assembled.

## 8. References, plurality, sets, and descriptions

### 8.1 Number-neutral references

`Referents<T>` is an abstract nonempty plural-reference domain. It is not
defined as `Set<T>`, a mereological sum, a group individual, or a mass. This
keeps ordinary predicate places neutral about singular versus plural reference
while allowing explicit mathematics when the graph requires it.

The minimum public algebra is:

```text
Singleton : T -> Referents<T>
Among     : Referents<T> × Referents<T> -> Content
Combine   : Referents<T> × Referents<T> -> Referents<T>
```

`Among` is a reflexive, transitive, antisymmetric subreference order after the
general singleton lift. `Combine` is its finite join: associative, commutative,
and idempotent; each operand is `Among` its combination; and the combination is
`Among` every common upper bound. There is no empty `Referents<T>` identity.
`Singleton`, `Among`, and `Combine` are effect-free and stable, so an
`Among`-based membership test does not by itself prevent a property from
satisfying `PureProperty`.

The format deliberately assumes no atoms, covers, distributivity of lexical
predicates, cumulative closure, collective/distributive default, or identity
between `Combine` and set union. Any such commitment is an explicit predicate
or operator.

### 8.2 Sets and counting

`Set<T>` is the ordinary free extensional set structure over `T`:

```text
SetOf : PureProperty<T> -> Set<T>
∈     : T × Set<T> -> Content
∪, ∩  : Set<T> × Set<T> -> Set<T>
×     : Set<A> × Set<B> -> Set<Tuple<(A B)>>
Card  : Set<T>|List<T> -> Cardinal
```

Exact counting is over an explicit singular basis supplied by source/model
metadata. It is never applied directly to `Referents<T>`. Infinite cardinality
is a `Cardinal` value; arithmetic comparisons with a finite natural retain
their standard mathematical meaning. A semantic model that supplies only an
approximate, vague, mass, or otherwise non-cardinal quantity uses its registered
quantity operator; until that row exists, projection fails with a registered
reason. `Card` is never used by analogy.

The basis is not determined by the broad sort `T` alone. A counting edge must
supply a source-licensed pure basis property and, when counting within a fixed
reference, a certified basis-membership relation. For example, counting dogs
within `$dogs` uses both `(gerku $x)` and `(Among $x $dogs)`; `Entity` by itself
does not determine which entity subreferences count as dogs or count units.
Calling that basis singular does not assert that every `Referents<T>`
decomposes into metaphysical atoms under `Among`. If the graph cannot supply
the property and membership evidence, the count is rejected with a registered
invalid-graph reason.

The extensional laws used by version 0 are:

```text
x ∈ SetOf(P)  ↔ P(x)
A = B         ↔ ∀x. x ∈ A ↔ x ∈ B
```

They apply only to pure properties. Independent effects are hoisted once to
an enclosing `Bind`, `Presuppose`, or `Supplement` before comprehension. An
effect which depends on the bound element and cannot be hoisted without a
semantic change makes that `SetOf` reduction unavailable, and projection fails
with a registered reason.

Mass readings and plural logic can be built with explicit sets, subreference,
group, part, collective, and distributive predicates. The format does not make
`Mass` a primitive merely because a source gadri historically received a mass
gloss.

### 8.3 `Refer`

```text
Refer : Property<Referents<T>> -> RefComp<Referents<T>>
```

`Refer P` contextually obtains a number-neutral reference and emits `P` of that
same reference as its descriptive side condition. The caller MUST NOT repeat
that side condition manually. Its continuation expansion is:

```lisp
(Bind (($r (Referents T) (Refer $P)))
  $body)
```

where the computation selects `$r`, contributes `($P $r)` through its declared
reference effect, and then evaluates `$body` once. `Refer` is neither iota nor
epsilon: those symbols conventionally suggest uniqueness or singular choice
that xorlo descriptions do not generally carry.

An ordinary xorlo `Refer` is a fixed reference in its force segment, with the
scope rule in section 6.3. It is not an existential whose scope is inferred
from its textual position. A nested `RefComp` required while evaluating `$P`
runs inside this reference computation unless the graph assigns that nested
effect its own legal outer host.

The main gadri lower compositionally. The first three supply a property to
`Refer`; the typical and stereotypical forms are their own irreducible reference
computations:

| Source family | Normal reference computation |
|---|---|
| `lo P` | `(Refer P)`, where `P` is veridical |
| `le P` | `(Refer (λ (($r (Referents T))) (skicu Speaker $r Audience P)))`; `P` is descriptive content, not asserted classification |
| `la N` | `(Refer (λ (($r (Referents Entity))) (Named N $r)))`; `Named` is the transparent standard `cmene`/`NameSign` expansion below with the current speaker as name-user, unless the graph supplies another explicit naming relation |
| `lo'e P` | `(Typical P)` |
| `le'e P` | `(Stereotypical Speaker P)` |

Their types are:

```text
Typical       : Property<Referents<T>> -> RefComp<Referents<T>>
Stereotypical : Referents<Entity> × Property<Referents<T>>
                -> RefComp<Referents<T>>
```

The explicit describer on `Stereotypical` prevents an irreducible operation
from silently assuming the speaker. Source `le` keeps the audience argument of
`skicu`; source `voi` uses `DescribedAs`. The version-0 `voi` projection records
a three-role description relation—describer, described referent, and
description property—and positively certifies that it has no audience role.
That semantic reduction contract, not the mere absence of a source operand,
licenses deletion of `skicu` x3.

There is no `Lo`, `Le`, `Relative`, or gadri-record constructor in normal form.

### 8.4 Relative clauses and associations

Relative-clause taxonomy is eliminated into ordinary composition:

- an inner restrictive `poi` in a description, such as
  `lo P poi Q`, conjoins its veridical clause property with the base
  description property before their one `Refer` computation;
- under the generalized-quantifier restriction analysis, a restrictive clause
  attached at that locus conjoins with the quantifier's restriction. In
  particular, an explicit outer
  quantifier separates the description's reference computation from the
  outer selection: a clause inside the description modifies the base
  reference property, while a clause outside its `ku` modifies the outer
  restriction. Lowering MUST NOT move a clause across that selection boundary;
- descriptive `voi` conjoins the host description property with
  `DescribedAs` of the same candidate instead of asserting the clause property;
  its transparent prelude definition is ordinary `skicu` with its audience
  place deleted under the registered three-role `voi` evidence above;
- supplementary `noi` contributes `(Supplement body side)` at its graph-owned
  anchor;
- multiple clauses combine with their actual logical or nonlogical connector;
- `goi` identity/assignment becomes `Let`, `Bind`, or the represented naming or
  association predicate, depending on its graph semantics.

Restrictive and nonrestrictive status is therefore visible in composition and
effect placement, not in an enum. Veridicality remains in the predicate that is
actually contributed.

The source position before the description selbri, such as
`lo poi ke'a Q ku'o P`, denotes the same outer attachment as the corresponding
post-`ku` clause; it is not the inner `lo P poi Q` attachment. Surface
`PA lo P ku poi Q` can support either a singular generalized-quantifier
restriction analysis or a prior subreference-selection analysis in semantic
profiles which distinguish them. The input graph MUST identify the selected
analysis and handler. If it does not, the renderer rejects it with a registered
invalid-graph reason; it never chooses from syntax, apparent distributivity, or
lexical plausibility.

A restrictive clause on an already assembled reference is not forced back
into the reference's original property. If the input graph identifies the
result with the original reference, ordinary predicate composition suffices.
If it instead represents selection of a subreference, the base computation is
evaluated once and `Among` relates the new reference to it. For a clause
property `$Q`:

```lisp
(Bind (($base (Referents T) $base-computation))
  (Refer
    (λ (($sub (Referents T)))
      (∧
        (Among $sub $base)
        ($Q $sub)))))
```

Some semantic profiles additionally give outer restrictive clauses a greatest
satisfying subreference reading, conventionally called maximality. That is not
a primitive or an unstated property of `Refer`. When the graph licenses it and
`$Q` is a `PureProperty<Referents<T>>`, its full reduction adds:

```lisp
(∀
  (λ (($candidate (Referents T)))
    (→
      (∧
        (Among $candidate $base)
        ($Q $candidate))
      (Among $candidate $sub))))
```

inside the new reference property. This states that every satisfying
subreference of `$base` is `Among $sub`; merely requiring `$sub` itself to
satisfy `$Q` would not express maximality. Together with `$Q $sub` and
`Among $sub $base`, antisymmetry of `Among` makes such a greatest
subreference unique if it exists. The algebra does not guarantee that it
exists. Failure to find one is ordinary reference-resolution failure, not a
projection failure; projection failure is reserved for failure to represent or
verify the graph's chosen reduction. If the clause property is effectful
and its effects cannot be hoisted once while preserving this duplicated use,
the maximal reduction is unavailable and projection fails with a registered
reason. When `$Q` contains a closure site or another shared identity, it MUST
be bound once as a function and applied at both sites; inlining two copies would
create distinct closure-site identities rather than duplicate one pure
property. `Among` and a second `Refer` are therefore printed only when the
input graph represents this distinct subreference computation, never merely
because a relative-clause syntax node occurred.

### 8.5 Set/group descriptions and referential connections

Set and group gadri create ordinary references to set or group objects through
dictionary predicates. The base reference is evaluated once. Schematically:

```lisp
; lo'i P
(Bind (($base (Referents T) (Refer $P)))
  (Refer
    (λ (($sets (Referents (Set T))))
      (selcmi $sets $base))))

; loi P
(Bind (($base (Referents T) (Refer $P)))
  (Refer
    (λ (($groups (Referents (Group T))))
      (gunma $groups $base))))
```

`le'i` and `lei` use the `le` property for `$base`; `la'i` and `lai` use the
`la` property. The outer `Refer` remains number-neutral, so it may refer to one
or more set/group objects. `SetOf` remains the mathematical comprehension
operator and is not the meaning of set gadri; `GroupOf` is unnecessary.

The four referential connections have distinct reductions:

| Source | Normal value |
|---|---|
| `X jo'u Y` | `(Combine X Y)` |
| `X joi Y` | one graph-described `gunma` object whose components are `(Combine X Y)` |
| `X ce Y` | one graph-described `selcmi` object whose members are `(Combine X Y)` |
| `X ce'o Y` | `(Singleton (List X Y))` |

For `joi` and `ce`, “one object” is an inner exact-one constraint on the result
reference's source-licensed singular basis, expressed with `SetOf`, `Among`, and
`Card`; it is not a `Counted` record. Plural operands require graph-supplied
flat component/member or ordered-element structure. Without it, the renderer
does not silently flatten one constructor but preserve opaque plural operands
in another; it rejects the graph with a registered invalid-graph reason.

Because `Set<T>`, `Group<T>`, and `List<T>` are first-order object subtypes,
their singleton or plural references can upcast to `Referents<Entity>` and fill
ordinary entity places. This does not unwrap a reference: there is no implicit
`Referents<Set<T>> -> Set<T>` or analogous group/list conversion. `Card` and
mathematical `∈` consume a raw set or list value; predicates such as `selcmi`,
`gunma`, and `cmima` consume the exact registered referential rows. A graph
which needs one particular mathematical object from a number-neutral reference
must represent the corresponding selection rather than coercing it.

`lu'i`, `lu'o`, and `vu'i` use the same set, group, and sequence relations on one
already assembled operand. `lu'a` is not a value constructor: it distributes
the containing property over members, for example:

```lisp
(∀
  (λ (($x T))
    (→
      (cmima $x $set)
      ($P $x))))
```

For an ordinary number-neutral operand, the antecedent uses `Among`. A crossing
whose membership/ordering relation is absent or whose empty result cannot fill
a nonempty `Referents<T>` place is rejected with a registered invalid-graph
reason.
The containing property must satisfy the same purity or one-time effect-hoisting
requirements as any other property placed under extensional iteration; `lu'a`
does not license duplicating a dynamic effect per member.

## 9. Logic and quantification

### 9.1 Logical operators

The canonical logical intrinsics are:

```text
¬ : Content -> Content
∧, ∨ : Content^n -> Content, n >= 2
→, ↔, ⊕ : Content × Content -> Content
∀, ∃ : Fn<(A1 ... An), Content> -> Content, n >= 1
```

Associative conjunction and disjunction flatten without reordering. A
one-operand occurrence contracts to that operand; zero operands do not print.
The dynamic rules in section 6 are part of these operators' meaning.

`∀` and `∃` receive an ordinary typed lambda; there is no separate quantifier
binder grammar. One lambda may bind several variables at the same quantifier
locus. Same-polarity nesting has the same truth conditions, but canonical
output preserves the graph's one-locus versus nested identity and source order.
All logical operators can be understood extensionally as
higher-order relations over content, but they remain primitives because their
truth and accessibility behavior is exact. Dictionary words print only when
the graph represents those dictionary predicates.

### 9.2 Generalized quantifiers

The common generalized quantifiers are transparent prelude functions with
these signatures:

```text
Some      : Property<T> -> GQ<T>
No        : Property<T> -> GQ<T>
Every     : PureProperty<T> -> GQ<T>
Exactly   : Natural × PureProperty<T> -> Fn<(PureProperty<T>), Content>
AtLeast   : Natural × PureProperty<T> -> Fn<(PureProperty<T>), Content>
AtMost    : Natural × PureProperty<T> -> Fn<(PureProperty<T>), Content>
MoreThan  : Natural × PureProperty<T> -> Fn<(PureProperty<T>), Content>
FewerThan : Natural × PureProperty<T> -> Fn<(PureProperty<T>), Content>
```

Natural arguments are nonnegative exact integers. Applying a `GQ<T>` to a
nuclear-scope property produces ordinary `Content`. The five cardinal helpers
encode the additional application precondition on their returned function's
nuclear-scope parameter as the nested `PureProperty<T>` refinement shown above;
this refinement follows the function through `Let` and other
identity-preserving bindings. `Every` permits an effectful nuclear scope because
it does not duplicate that scope, but its restriction is pure because the
import and conditional each use it. These are checker metaconstraints, not new
printable type constructors. After refinement erasure the cardinal return type
is still structurally `GQ<T>`; an application which fails its retained purity
precondition makes projection fail with a registered reason rather than
evaluating the displayed `SetOf` expansion unsoundly.

For a restriction `$P` and nuclear scope `$Q`, their definitions are:

```text
Some $P      = (λ (($Q (Fn (T) Content)))
                 (∃ (λ (($x T)) (∧ ($P $x) ($Q $x)))))
No $P        = (λ (($Q (Fn (T) Content)))
                 (¬ (∃ (λ (($x T)) (∧ ($P $x) ($Q $x))))))
Every $P     = (λ (($Q (Fn (T) Content)))
                 (Presuppose
                   (∃ (λ (($x T)) ($P $x)))
                   (∀ (λ (($x T)) (→ ($P $x) ($Q $x))))))
Exactly n $P = (λ (($Q (Fn (T) Content)))
                 (= (Card (SetOf
                      (λ (($x T)) (∧ ($P $x) ($Q $x))))) n))
AtLeast n $P = (λ (($Q (Fn (T) Content)))
                 (≥ (Card (SetOf
                      (λ (($x T)) (∧ ($P $x) ($Q $x))))) n))
AtMost n $P  = (λ (($Q (Fn (T) Content)))
                 (≤ (Card (SetOf
                      (λ (($x T)) (∧ ($P $x) ($Q $x))))) n))
MoreThan n $P = (λ (($Q (Fn (T) Content)))
                  (> (Card (SetOf
                       (λ (($x T)) (∧ ($P $x) ($Q $x))))) n))
FewerThan n $P = (λ (($Q (Fn (T) Content)))
                   (< (Card (SetOf
                        (λ (($x T)) (∧ ($P $x) ($Q $x))))) n))
```

These equations are type-parameterized specification schemas. Section 14.1
installs each instantiated function through the implicit prelude. The cardinal
helpers require the combined `$P`/`$Q` property to satisfy the `PureProperty`
constraint of `SetOf`. They also use the source/model's
registered singular counting basis. If the required effects cannot be hoisted
once, or the basis is absent, that transparent helper is unavailable and
projection fails with a registered reason rather than duplicating effects.
For `Every`, the same one-time hoisting rule applies to its duplicated
restriction `$P`; its nuclear scope `$Q` may remain effectful because the
definition does not duplicate one application. For the cardinal helpers it
applies to both `$P` and `$Q` through their combined comprehension property.

Source `ro` uses `Every`, whose definition includes a projective
nonempty-restrictor presupposition. The primitive `∀` is the nonimporting
mathematical universal. There is no separate `(Import Projective)` node and no
unstated import default.

For example, `((Every P) Q)` elaborates in effect to:

```lisp
(Presuppose
  (∃ (λ (($x T)) ($P $x)))
  (∀ (λ (($x T)) (→ ($P $x) ($Q $x)))))
```

with all required singleton lifts and dynamic placement retained. This also
preserves importing behavior under negation.

A source restriction is conjoined into `$P` before constructing the `GQ`.
There is no `Restrict` constructor: applying an opaque completed `GQ` to an
extra nuclear-scope condition would not in general preserve importing behavior,
so the renderer does not pretend that it recovered the original restrictor.

### 9.3 Inner and outer cardinality

An inner count constrains one fixed number-neutral reference through its
source-licensed singular-member property. An outer count quantifies singular
satisfiers. These are not interchangeable.

The canonical mathematical reduction uses `SetOf`, `Card`, membership, and
ordinary predicates. A count is never inferred from the apparent number of
referents in a `Combine` value.

`lo no P` follows the exact-zero generalized-quantifier path when the source
semantics supplies one; it does not attempt to construct an empty
`Referents<T>`, which is impossible by definition.

### 9.4 Witness export

A shared generalized-quantifier application retains its own content identity
as well as the identities of its `GQ` and nuclear-scope property. A successful,
graph-exporting application may supply a witness reference through the
dependent signature schema:

```text
Witnesses : (run : Content where run is directly GQ<T> applied to Property<T>)
            -> RefComp<Referents<T>>
```

`Witnesses $run` is legal only in a dynamic continuation dominated by exactly
one successful performance of that bound application and when the semantic
graph exports its selection identity. It does not execute the quantifier or
duplicate its truth condition. Reperforming the same `$run`, or reaching it
through a branch which does not guarantee that unique success, makes retrieval
ambiguous and therefore invalid. The type/effect checker tracks application
identity, static witness type, performance occurrence, and success capability
through identity-preserving `Let` bindings, acts, conjunction, and `Do`.

The uniqueness test is made at the retrieval site over successful performance
occurrences which dominate that site on every reaching execution path. A later
performance is irrelevant; two earlier dominating performances are ambiguous,
and a branch with zero or multiple such performances does not establish the
capability.

The checker implements this as a finite performance-flow analysis after act
identity, `Do`, `Perform`, and `PerformUtterance` planning but before rendering.
Every node carries its lexical quantifier/lambda and reusable-act region stack.
For each bound run identity, path flow uses the abstract counts `Zero`, `One`
and `Many`: sequential performance adds a site, branches join only when they
name the same single occurrence, and a loop, repeated act performance, or
escaped multiplicity region widens to `Many`. A retrieval is legal exactly when
all incoming paths carry `One` for the same successful occurrence and its
region stack is still in scope at the retrieval. A retrieval inside one dynamic
quantifier instantiation may therefore use a run performed earlier in that same
instantiation; the occurrence cannot escape to a site outside that multiplicity
region. Unknown control flow widens rather than assuming uniqueness.

```lisp
(Let (($gq (GQ Entity)
        (Exactly 3 (λ (($x Entity)) (gerku $x)))))
  (Let (($scope (Fn (Entity) Content)
          (λ (($x Entity)) (bajra $x))))
    (Let (($run Content ($gq $scope)))
      (Do
        (Assert $run)
        (Bind (($dogs (Referents Entity)
                (Witnesses $run)))
          (Assert (tatpi $dogs)))))))
```

Every singleton member of a returned reference satisfies the run's restriction
and nuclear scope. The graph's exported selection additionally guarantees the
cardinality or completeness contract of that quantifier family. `Witnesses` is
a verified selection-to-reference bridge, never a general coercion from
`Set<T>` to `Referents<T>`.

In specification metanotation, after binding
`$w <- Witnesses($run)` for `$run = ($gq $Q)` with restriction `$P`, the minimum
contract is:

```text
Among(Singleton(x), $w) -> P(x) ∧ Q(x)
```

The quantifier family's registered export contract may add exact cardinality or
exhaustivity, but may not weaken this membership law.

Retrieval before successful execution, through a branch in which execution is
not guaranteed, or across an intensional/opaque boundary is ill-scoped. A
request is also invalid unless successful execution guarantees a nonempty
selection: for example `Some`, importing `Every`, positive `Exactly`/`AtLeast`,
and `MoreThan` a natural may qualify, while `No`, `Exactly 0`, `AtMost 0`, and
other zero-compatible applications do not. The graph must record the exported
selection; truth conditions alone never invent one. An invalid source request is
rejected with a registered invalid-graph reason rather than making the renderer
try to construct an empty `Referents<T>`.

### 9.5 Simultaneous termsets

A genuinely simultaneous termset whose graph licenses the
**coordinate-closed complete-product** profile does not require a `PolyQuant`
primitive. For each participant, let `Pi : PureProperty<Ti>`, let `Qi` be its
source generalized-quantifier family, bind `Si : Set<Ti>`, and let the pure
polyadic nuclear relation be `R : Fn<(T1 ... Tn), Content>`. Define:

```text
Ei(xi; S-i) =
  Pi(xi) ∧
  ∀ x1 ... x(i-1) x(i+1) ... xn.
    (∧j≠i xj ∈ Sj) → R(x1,...,xn)
```

The normal form existentially binds all `Si` at one same-force multi-parameter
`∃` and, for every coordinate, conjoins its one canonical direct participant
constraint `Di` with:

```text
∀ xi. xi ∈ Si ↔ Ei(xi; S-i)
```

The closed `Di` spellings are:

```text
Some          Di = (> (Card $Si) 0)
Exactly n     Di = (= (Card $Si) $n)
AtLeast n     Di = (≥ (Card $Si) $n)
MoreThan n    Di = (> (Card $Si) $n)
Every         Di = (= $Si (SetOf $Pi))
```

For importing `Every`, its independent
`Presuppose(∃ xi. Pi(xi), ...)` is hosted as specified below. These direct
constraints are the normative output of the registered coordinate-closed
reduction. They use the biconditional's consequence `Si ⊆ Pi` and the
extensional equality of the selected comprehension with `Si`; they are not
mere beta reductions. The renderer MUST NOT instead print the equivalent
generalized-quantifier application for this profile.

The biconditional makes each selected set the maximal coordinate of a complete
rectangle. This is stronger than merely saying that some selected Cartesian
product satisfies `R`: if four dogs all relate to the same two people, an
`Exactly 3` by `Exactly 2` rectangle exists, but the coordinate-closed reading
is false because the dog coordinate contains four. That strength is deliberate
only when the graph or a verified source reduction records it; equal scope
alone does not entail maximality.

The selected-set equations are mutually referential, but the one multi-
parameter existential introduces no asymmetric participant scope order. This
schema is licensed only for a graph-recorded coordinate-closed profile with
pure `Pi` and `R`, supported `Qi`, every singular counting basis, and one
polyadic nuclear scope. The supported participant families are `Exactly n`
and `AtLeast n` for `n > 0`, `MoreThan n`, `Some`, and importing `Every`; naming
a family never substitutes for the required profile evidence.

This one-locus multi-parameter binder is distinct from the deliberately nested
existentials used for a repeated tense path such as `pu pu` in section 10.4.
Those binders encode successive path-node dependencies, not simultaneous
participant quantification; `samples.md` sections 4 and 12 show the two graph
shapes separately.

For an importing `Every` participant, the `Presuppose` associated with `Di`
obeys section 6.4 independently of the selected-set existential. Its trigger
mentions `Pi` but not `Si`, so the supported schema hosts that trigger at the
nearest dependency-legal force handler outside the selected-set binder. If a
candidate trigger depends on `Si`, another selected coordinate, or branch-local
data, this version-0 termset reduction is not licensed and projection fails with
a registered reason.

Downward-entailing, zero-compatible, non-cardinal, partial-product,
complete-but-not-coordinate-closed, effectful, or otherwise unsupported
profiles make projection fail with a registered reason. No participant witness
escapes this existential in version 0, so a graph which requires later anaphora
to one of the `Si` also fails: version 0 specifies no joint termset-witness
operation, and that is a tracked spec gap under section 14.4 rather than
expressiveness this version has. If the graph has already collapsed equal scope
into an ordered individual nest, the renderer does not attempt to reconstruct
simultaneity heuristically.

## 10. Modals, nonlogical connections, and events

### 10.1 Modal normalization

Every tag is expanded by a versioned, semantics-preserving lexical or
compositional reduction. The result is one or more ordinary predicates attached
by the source's ordered `Joi` connection at the recorded locus, with logical or
other registered connectors retained inside a compound tag. Each predicate's
own places are filled normally. BAI, explicit `fi'o`, tense, space, aspect,
recurrence, and actuality all follow this rule; their differing source families
select different predicate and place maps, not different semantic ontologies.

```lisp
(∃
  (λ (($e Eventuality))
    (Joi
      (klama Speaker :Eventuality $e)
      (pilno :2 This $e))))
```

`sepi'o` and direct `fi'o pilno` may map their source operands to different
places; the verified modal expansion supplies that map. They still use the same
`pilno` root and contain no `Modal` or modal-valued `At` node.

The displayed `sepi'o` candidate fills `pilno` x3 and therefore commits to the
purpose reading “used for the host event.” Its shipped tag row must record
evidence for that reading; sharing an event type or occurring beside the host
does not by itself license the place map.

The host event fills a modal predicate place only when the semantic graph and
the versioned expansion table identify that exact link. The renderer MUST NOT
choose the first place whose type happens to admit `Content`, `Eventuality`, or
another plausible value. An arbitrary `fi'o broda` with no licensed host-event
link remains simply `(Joi host (broda payload...))`, with its other places
closed by the ordinary rules.

Compound tags preserve their actual logical or nonlogical connector and
negation. Elided tag operands receive the same explicit/default treatment as
other predicate places. Arbitrary `fi'o` accepts any graph-valid predicate term,
including one assembled with `be`, functions, or reified content.

Multiple tags remain attached at their graph-recorded loci and preserve source
order among effects at one locus. A tag never crosses an act, quotation,
reification, or other force boundary merely to make the output shorter.
Expressive or metalinguistic modifiers are acts or displayed-content relations,
not additional `Joi` operands.

### 10.2 `Joi`

`Joi` is an ordered nonlogical connector with homogeneous overloads:

```text
Joi : Content^n -> Content, n >= 2
    | Discourse^n -> Discourse, n >= 2
```

It retains the graph's one nonlogical connection locus and operand order; it
does not invent a more specific mixture, simultaneity, causation, or logical
conjunction relation. Content and discourse operands never mix in one call.
Its context flow is the rule in section 6.

At a discourse locus, an `Act` operand crosses explicitly with `Perform` and a
`TranscriptEntry` crosses explicitly with `PerformUtterance`; an already
`Discourse` operand remains as written. The resulting operands are therefore
homogeneous without a mixed `Joi : Performable^n` overload:

```lisp
(Joi
  (Perform $act)
  (PerformUtterance $entry)
  $discourse)
```

Those wrappers are required here because `Joi` is an ordinary
`Discourse`-typed position, not the implicit `Smusni`/`Do` performance spine.

When the source nonlogical connective denotes a different registered
operation—interval, union, sequence, mixture, respective pairing, or another
typed collection operator—the corresponding typed operator prints instead.
A nonlogical family with no registered typed operator makes projection fail
with a registered reason.

### 10.3 Event places and facets

The distinguished `:Eventuality` place is separate from numbered lexical
places. Every event-licensed root or relation former carries the open slot
`(Eventuality (Referents Eventuality))` until it is filled or closed. A raw
`Eventuality` singleton-lifts into that slot; a `Referents` value of a declared
event subtype upcasts covariantly. There is no reverse conversion. The slot
prints whenever an event identity is shared, constrained, targeted, or
otherwise cannot be hidden by `Close`.

A modal-introduced event existential is placed at the smallest lexical body
that contains every use of the event while remaining inside its force and
intensional boundaries. A graph-owned wider or narrower event scope overrides
that default when legal. The event variable never becomes accessible merely
through dynamic context; every use lies inside its printed `λ`.

Event properties are ordinary predicate terms of the event:

```lisp
(∃
  (λ (($e Eventuality))
    (∧
      (klama Speaker :Eventuality $e)
      (purci $e))))
```

No generic event-property record prints. Each represented facet must lower to
a registered lexical predicate or typed intrinsic with a declared event
signature. An unregistered facet makes projection fail with a registered reason rather
than being dropped.

### 10.4 Tense, space, aspect, recurrence, and actuality

These families lower to ordinary lowercase Lojban predicates of the shared
event, its anchor, or an explicit interval/path value. A reduction MAY use more
than one predicate, a lambda, a quantifier, or another kernel operation. A
readable PascalCase helper MAY be retained only as a transparent prelude
binding whose complete `Let` definition uses those ordinary predicates and
kernel operations. It is not a primitive merely because the source item is a
cmavo rather than a brivla.

For example, the standard temporal reductions use the lexical rows of `purci`,
`cabna`, and `balvi`: `pu` relates the clause event as x1 of `purci` to its
anchor as x2; `ca` analogously uses `cabna`; `ba` uses `balvi`. Event contours,
recurrence, paths, interval properties, and actuality may require compound
definitions rather than approximate one-word glosses. Each shipped table entry
therefore records the complete typed reduction and its evidence, not a minted
PascalCase predicate name.

The directly lexical spatial directions use the same shape. The verified tag
reduction supplies every semantically fixed role rather than delegating a cmavo
default to generic `Close`. For the CLL speaker-relative axial readings, the
host event or location fills x1, `Here` supplies the speech-location origin in
x2 where that row requires it, and `Speaker` supplies the orientation standard
in x3; an explicit `ma'i` replaces that orientation value. Other rows, including
absolute or landmark-dependent directions, carry their own exact origin/frame
map and are not inferred by analogy. This follows CLL sections 10.2 and 10.8,
which make the unmarked journey start at the speaker's location and define
left/right in the speaker's reference frame:

| Source directions | Lowercase relation pair |
|---|---|
| `ca'u` / `ti'a` | `crane` / `trixe` |
| `zu'a` / `ri'u` | `zunle` / `pritu` |
| `ga'u` / `ni'a` | `gapru` / `cnita` |
| `be'a` / `ne'u` | `berti` / `snanu` |
| `du'a` / `vu'a` | `stuna` / `stici` |

Other FAhA members are not forced into this table by analogy. For example,
movement, traversal, proximity, containment, and direction-without-position
need their own exact paths through roots such as `muvdu`, `pagre`, `jibni`,
`nenri`, or `farna` and all of those roots' independently represented places.
An unrelated ordinary use of one of the lowercase spatial predicates may still
omit its x2 or x3 and receive a lexical `Context` through `Close`; that is not
the semantics of the registered FAhA default.

`mo'i` is not an event-facet primitive. It transforms exactly the following
spatial step, and that transformed step expands through ordinary motion and
spatial predicates. The graph and the verified reduction row must say whether
the motion is the host event, a distinct concurrent event, or an event whose
result bears another represented relation to the host. `Joi` records the source
tag attachment; it does not itself assert simultaneity or causation.

Place deletion is essential to these reductions when a verified row says the
derived relation semantically lacks one of `muvdu`'s destination, origin, or
route roles. Such a role is removed with `DropPlace`; mere absence of a source
operand still receives ordinary contextual closure. Conversely, a role needed
to connect the motion to its direction, distance, landmark, or route remains
present. There is therefore no one fixed projection of `muvdu` for every
`mo'i` step.

For an axial step whose verified reduction represents motion from an origin to
a destination, `MotionVector` is the transparent prelude helper defined in
section 14.1. Let `R` be the corresponding lowercase relation from the table
above. A same-event reduction has this shape:

```lisp
(Joi
  (host :Eventuality $event)
  (MotionVector
    $event
    $mover
    (λ (($destination (Referents Entity))
        ($origin (Referents Entity)))
      (R $destination $origin $frame))))
```

`R` is specification metanotation and is replaced by `crane`, `trixe`,
`zunle`, `pritu`, `gapru`, `cnita`, `berti`, `snanu`, `stuna`, or `stici` in
an actual document. `MotionVector` retains `muvdu` x1, x2, and x3 because the
mover and the displacement endpoints are all used, fills the distinguished
`:Eventuality` slot with `$motion`, and deletes x4 because this helper's
registered displacement contract positively states that the derived relation
has no route role. This is semantic absence evidence, not an inference from an
omitted route operand. The graph must identify `$mover`; the renderer never
equates it with the host predicate's x1 merely because both are available.
CLL's `mi mo'i ca'uvu citka ...` can describe eating in a moving airplane, so
grammatical x1 is not a general mover rule. The default frame is the utterance's
speaker-oriented frame, so a frame-dependent axial template prints `Speaker`;
an explicit `ma'i` supplies the represented replacement.
`NotationDefault` in the source disposition ledger suppresses the redundant
input-model default field, not the semantically required `Speaker` operand in
the expansion. This `mo'i` rule does not change ordinary unrelated spatial
predicates, whose omitted frame place may still receive its own `Context` value
through `Close`.

If the motion has its own event `$motion`, `MotionVector` receives that event
and the internal tag content additionally contains the graph's relation to the
host. A simultaneous reading uses `(cabna $motion $event)`; a result-dependent
reading uses its represented result or causal relation instead. The renderer
does not introduce either event identity or relation merely from the spelling
`mo'i`.

Other motion directions use other projections. A graph-resolved heading with
no represented destination, origin, or route can use:

```lisp
(∧
  ((DropPlace (DropPlace (DropPlace muvdu 2) 3) 4)
    $mover
    :Eventuality $motion)
  (farna $direction $motion $frame))
```

Because deletion preserves surviving labels, the three literals continue to
name `muvdu` x2, x3, and x4 at their respective current rows. This heading-only
reduction is available only when the graph supplies a first-class direction
accepted by `farna`; it does not claim arrival at a target and is not the
arrival reading that CLL assigns to `mo'i fa'a`. A passage reading may instead
use a projection of
`pagre`, while an along/around reading must retain or construct a route and
constrain that route. `fa'a`, `to'o`, `ne'i`, `zo'i`, `ze'o`, `zo'a`, `pa'o`,
`re'o`, `te'e`, and `ru'u` therefore require individual verified table rows;
they are not obtained by substituting a different static relation into the
axial schema. Spatial distance constrains the same endpoint pair or route in
the selected schema; `vi` and `vu` may use standard `jibni` and `darno`, while
no exact baseline predicate for the intermediate `va` reading is assumed. If
the graph and table do not determine the needed mover, event relation,
endpoint, path, landmark, or frame links, that transformed step is the failing
edge and projection fails with a registered reason.

The heading-only reduction is licensed only by an explicit graph/reduction
classification that destination, motion origin, and route roles are all
semantically absent. The `$frame` in `farna` x3 is the direction relation's own
origin/frame role; matching its type to `muvdu` x3 does not identify the two
roles. If the graph does identify a motion origin, the reduction must retain
and fill `muvdu` x3 instead of using this projection. The registered evidence
id for the three deletions is `smusni.motion-heading.no-endpoints-or-route`.

Repeated directions compose as paths. For example, `pu pu` introduces the
intermediate reference needed for “before a time which is itself before the
anchor”; it is not flattened into two unrelated predicates of the host event:

```lisp
(∃
  (λ (($event Eventuality))
    (∃
      (λ (($middle Eventuality))
        (Joi
          (broda :Eventuality $event)
          (purci $middle Now)
          (purci $event $middle))))))
```

ZAhO contours similarly use the host event as the process and an explicit or
contextual checkpoint as anchor. Their exact event-boundary ontology is not
recoverable merely by choosing a root such as `krasi`, `cfari`, or `fanmo`:
argument direction, checkpoint identity, and the relation between a boundary
and the host must all be present in the verified reduction. Until an individual
row supplies that structure, `co'a`, `co'u`, natural endings, pauses,
resumptions, anticipation, aftermath, and excessive continuation make projection
fail with a registered reason. ROI and TAhE may require a set of distinct event tokens,
`SetOf`, `Card`, ordering, and a lowercase relation such as `rapli`, `krefu`,
`dikni`, `cafne`, or `tcaci`. The BPFK-style `n roi` and `n re'u` expansions
through `rapli` and `krefu` are admissible only with the tagged interval and
ordinal/count places preserved. These are transparent higher-order reductions,
not reasons to keep opaque contour or recurrence constructors. A row is
unavailable until its event identity, counting interval, and any
agent/participant policy are exact.

For example, `mi pu citka` has an event explicitly related to the utterance
time and joined to the host predication at the tag locus:

```lisp
(∃
  (λ (($e Eventuality))
    (Joi
      (citka Speaker :Eventuality $e)
      (purci $e Now))))
```

Anchor paths and intervals remain first-class when shared or independently
modified. The renderer does not choose a nearby lexical word by English gloss,
merge actuality with assertion, or replace a source logical connector within a
compound tag by `Joi`. A table row is licensed only by an exact reduction;
an unsupported facet receives its own disposition entry and makes projection
fail with a registered reason.

The important exceptions are semantic rather than grammatical:

- `ca'a` contributes ordinary `(fasnu event)` content. It never contracts with
  `Assert`, because actuality can occur under negation, questions, quotation,
  and intensional operators.
- `ka'e` uses the transparent `InnatelyCapable` helper defined in section 14.1.
  Its host-property operand abstracts both the host's x1 bearer and its
  eventuality while retaining every other filled argument. The helper uses
  `kakne` for capability and `jinzi` for the source's stronger innate reading;
  it is not an opaque modal primitive. The reduction is available only when
  the host has a recoverable x1 and event property. `pu'i` and `nu'o` require
  more than adding or negating an unrelated `fasnu` predication: a reduction
  must identify the same capability and the corresponding demonstrated or
  unrealized event. Until that complete relation is registered, each makes
  projection fail with a registered reason.
- `ki` is source elaboration only and is eliminated before rendering by
  explicit reuse. A sticky tag's fully lowered relation
  path is applied again at every inheriting event; a later explicit tag extends
  that path in source order. The renderer binds a shared anchor or intermediate
  path value with `Let` for an inert value or `Bind` for a `RefComp`, over the
  smallest body containing every reuse, including around `Do`, only when graph
  identity makes that sharing observable. Bare `ki` resets subsequent source
  elaboration to the current deictic ground, and a subscripted `ki` resolves to
  reuse of the graph's ordinary bound anchor or constant. There is no
  saved-anchor type. None of
  these cases leaves a `Ki`, context-update, or discourse-state node in normal
  output. A `manri` predication would merely describe a standard and therefore
  is not the expansion of `ki`.
- `cu'e` binds an open question over an exactly typed tag function. `fe'e` is a
  temporal-to-spatial transformation schema. Neither is an event predicate.

## 11. Abstractions and higher-order values

### 11.1 Properties and relations

Source `ka` and relation abstractions are lambdas whenever they merely abstract
ordered `ce'u` places:

```lisp
(λ (($x (Referents Entity)))
  (melbi $x))
```

Multiple `ce'u` occurrences follow their graph-owned parameter identities and
order. Free variables remain free only within the surrounding lexical scope;
the final document has no unbound variables.

A first-class place permutation is likewise a lambda, as described in section
4.5. No `Property` or `Relation` record is needed around the function.

### 11.2 Event abstractions

An event abstraction is an ordinary reference computation whose property binds
the event explicitly:

```lisp
(Refer
  (λ (($events (Referents Eventuality)))
    (klama Speaker :Eventuality $events)))
```

The caller uses `Bind` exactly as for any other `RefComp`. More specific event
sorts replace `Eventuality` in the property with `Achievement`, `Process`,
`Activity`, `State`, or `Experience`; their one-way upcast permits the reference
at the root's event place. Therefore `EventOf`, `AchievementOf`, and `StateOf`
are not primitives. Process, activity, and experience abstractions use the
crossings in section 11.3 only when their second Lojban places (stages,
components, or experiencer) are independently represented; the one-place case
remains the ordinary `Refer` shape.

### 11.3 Other abstraction crossings

The irreducible content-to-object crossings are:

```text
Reify              : Content -> Proposition
Measure            : Content × Referents<Scale> -> Referents<Amount>
TruthValue         : Content × Referents<Epistemology> -> Referents<TruthValue>
ExperienceOf       : Content × Referents<Entity> -> Referents<Experience>
ProcessOf          : Content × Referents<Eventuality> -> Referents<Process>
ActivityOf         : Content × Referents<Eventuality> -> Referents<Activity>
Concept            : Content × Referents<Entity> -> Referents<Concept>
Abstract           : Content × Referents<T> -> Referents<AbstractNature>
SentenceSign       : Content -> Sign<Sentence>
```

For the seven two-operand crossings, the trailing operand is declared
contextual-defaultable. Its omission is exact notation sugar for a local
`Bind` of a fresh, correctly typed `Context` computation followed by the
full-arity application. An explicit graph operand always prints. Each omitted
operand introduces a distinct contextual computation unless graph identity says
otherwise. Missing source information that is not semantically a contextual
default is rejected with a registered invalid-graph reason rather than using
this sugar.

Thus `Measure content` is not a second overload or primitive. It elaborates as:

```lisp
(Bind (($scale (Referents Scale) Context))
  (Measure content $scale))
```

The same rule eliminates `MeasureOn`, `TruthValueBy`, `ExperienceFor`,
`ProcessThrough`, `ActivityThrough`, `ConceptFor`, and `AbstractAs`.

## 12. Questions and answers

### 12.1 Query values

```text
Polar : Content -> Query<()>
OpenQ : Fn<(A1 ... An), Content> -> Query<(A1 ... An)>
```

A question's variable domain is explicit in the lambda type. Ordinary arguments
use their value type; relation questions use the exact `Fn` or open-row
`PredTerm` required at the site; place questions use `PlaceOf`. Connective,
tense/modal, mathematical-operator, attitude, and quantity questions print
normally only when the versioned registry supplies an exact higher-order or
value type for every answer. There are no opaque `Connective`, `TenseModal`, or
`MathOperator` sort atoms. A graph which records only one of those coarse model
sorts is rejected with a registered invalid-graph reason rather than making an
uncheckable question variable.

A place question binds `PlaceOf<R,T>` when its finite compatible candidate row
is derivable at the unique `At` host, or `PlaceOf<R,T,C>` when the graph narrows
that row or a shared variable requires the domain to be stated.
A mixed multiple question uses a multi-parameter lambda and an answer tuple in
the same order.

A graph truth-slot question lowers to `Polar content`. `OpenQ` is used only when
the graph binds a value, relation, place, operator, or other nonempty answer
domain; it does not manufacture a Boolean answer variable for a polar question.

`Ask` turns a query into a direct question act. `QuestionOf` turns it into an
inert question object. They do not assert or answer it.

### 12.2 Answers

`Answer` always states the answer selection profile:

```text
Answer : Query<A> × AnswerSelection<A> -> Content
```

The closed selection forms are:

```text
(PolarAnswer Yes|No|Unknown)
(TupleAnswer (Tuple value...) [Exhaustive|MentionSome])
ContextualAnswer
(ContextualAnswer Exhaustive|MentionSome)
UnresolvedAnswer
```

`ContextualAnswer` is the contextually true answer operator used when the
semantics commits to answerhood but does not encode the selected value. It does
not pretend the graph recorded a polarity or tuple. `UnresolvedAnswer` is
permitted only when unresolved answerhood is itself a semantic value; an
implementation which merely cannot prove the projection takes the section-16
failure route instead.

The exhaustivity operand is optional, and its omission is the canonical
spelling of the predominant case: an answer whose exhaustivity is genuinely
undetermined by the represented semantics. An omitted operand commits to the
answer with the exhaustivity conjunct simply absent from its truth conditions —
not a weaker or disjunctive reading, not a defaulted value later resolvable
from context, and not a claim that the question is unanswered. It therefore
remains distinct from `UnresolvedAnswer`, which says that unresolved
answerhood is itself what the graph represents.

Omission is not a hedge for a producer that could have determined the reading.
It is the correct spelling when the represented semantics genuinely contains no
exhaustivity. Lojban supplies no surface exhaustivity marker for the
indirect-question particle `kau`, focus-`kau` supplies the answer's value
rather than its exhaustivity, and choosing from the embedding predicate would
be a lexical heuristic this format forbids. Bare `ContextualAnswer` is
consequently the only answer selection tersmu emits for a bare `kau`, and
`lo du'u ma kau se bilga` reaches an ordinary `Answer` rather than a projection
failure.

`Exhaustive` and `MentionSome` are the two closed `AnswerExhaustivity`
literals, printed only for the uncommon committed case, and they remain in the
notation for expressive completeness. Smusni is a language rather than a
transcript of one producer's output: a hand-written document, another
producer, or a future Lojban marker or BPFK ruling can commit to either
reading, and the format must be able to write what they mean.

The tuple type parameters of `ContextualAnswer` and `UnresolvedAnswer` are
inferred from the `Query<A>` paired with them by `Answer`; these constructors
do not independently choose an answer tuple type.

Query-local variables never escape through `Answer`. A de-re reference inside
an embedded question escapes only through a legal graph-owned host.

## 13. Math, quantities, sequences, and quotation

### 13.1 Mathematical values

Common exact operators use conventional symbols:

```text
= ≠ < ≤ > ≥ + − × ÷ ∈ ∪ ∩
```

`Set`, `List`, `Tuple`, and `Interval` construct typed mathematical values. In
value position:

```text
Tuple : A1 × ... × An -> Tuple<(A1 ... An)>
Card  : Set<T>|List<T> -> Cardinal
Interval : T × T × EndpointInclusion × EndpointInclusion -> Interval<T>
           for a registered ordered T
```

The complete version-0 ordered-type registry is `Number`, `Natural`, and
`Cardinal`. `<`, `≤`, the transparent `>` and `≥` preludes, and `Interval` are
licensed only when both operands or endpoints have one common type from that
set after the implicit conversions of section 3.1; type inference chooses the
unique least such target in the conversion preorder. `Natural` uses the order
inherited from `Number`; `Cardinal` has its standard cardinal order, including
infinite cardinal values. No order is inferred for any other type. A comparison
or interval over another type makes projection fail with a registered reason
unless a later format version explicitly registers that order.

`Open` and `Closed` are the two endpoint-inclusion literals. The first applies
to the first endpoint and the second to the second endpoint. Centered,
unordered, or otherwise differently structured intervals require an exact
registered reduction; until one exists, projection fails with a registered
reason. Endpoint order is never guessed.

The same `Tuple` head in a type expression is distinguished by the grammar's
type position. The renderer always uses U+2212 `−` for subtraction; ASCII
hyphen remains an atom character and is never the subtraction operator.

`≠`, `>`, `≥`, `∪`, and `∩` are transparent prelude functions:

```text
≠ : T × T -> Content
>, ≥ : T × T -> Content, for a registered ordered T
∪, ∩ : Set<T> × Set<T> -> Set<T>

≠ a b = (¬ (= a b))
> a b = (< b a)
≥ a b = (≤ b a)
∪ A B = (SetOf (λ (($x T)) (∨ (∈ $x A) (∈ $x B))))
∩ A B = (SetOf (λ (($x T)) (∧ (∈ $x A) (∈ $x B))))
```

The equations are typed schemas installed through the implicit `Let` prelude.
`=`, `<`, `≤`, `+`, `−`, `×`, `÷`, `∈`, `SetOf`, and the data constructors remain
primitive because the smaller kernel contains no equally general definition
for them.

`ZipWith` represents respective application to ordered
collections:

```text
ZipWith : Fn<(A1 ... An), Content>
          × List<A1> ... × List<An> -> Content
```

All input lists must have the same finite length. A represented mismatch is
rejected with a registered invalid-graph reason; `ZipWith` neither truncates nor
pads.

Arrays, powers, nondecimal bases, subscripts, operator denotation, and
questioned operators print only through a registered typed math constructor.
An unknown operator is never emitted as an unregistered atom; the smallest
unknown math value makes projection fail with a registered reason.

The registered `ZipWith` form returns `Content`. A respective construction
whose result is a non-content collection requires its own exact registered
operator. Version 0 registers none; that is a tracked spec gap under section
14.4, and projection fails with a registered reason rather than overloading
this signature.

### 13.2 Quantities

Exact cardinal quantities reduce to set/cardinality mathematics. A non-cardinal
quantity prints normally only when a versioned entry gives an exact lexical or
transparent-prelude reduction with the complete scale, comparison set, and
property. Version 0 has no generic `Amount`, `Extent`, `Frequency`, `Portion`,
or `Ordinal` constructor: those names would merely repackage the model record.

Approximate, indefinite, comparative, and vague quantities therefore make
projection fail with a registered reason until such a reduction is registered.
They MUST NOT be rounded into an exact `Number` or cardinal comparison.

### 13.3 Quotation and signs

```text
OpaqueQuote     : Text -> Sign<Opaque>
StructuredQuote : TranscriptEntry|Discourse -> Sign<Structured>
NameSign        : Text -> Sign<Name>
SentenceSign    : Content -> Sign<Sentence>
```

Word, letteral, connective, math-expression, and text signs use a `Sign` token
plus ordinary registered facts when those facts completely represent their
structure. Version 0 does not introduce generic `WordSign`, `LetteralSign`, or
similar record wrappers. A raw sign value of one of those kinds whose structure
cannot be expressed by the four constructors above and token facts has no
version-0 route; that is a tracked spec gap under section 14.4, and projection
fails with a registered reason. Opaque quotation preserves exact quoted text and
does not claim an internal interpretation.

## 14. Registered normal forms

### 14.1 Callable-name registry and transparent prelude

Every intrinsic PascalCase atom or mathematical glyph is an irreducible
primitive/constant or a transparent value/function in the version-0 prelude.
The following primitive groups are closed for normal version-0 output.
Polymorphic type variables are inferred from operands.

| Group | Primitives |
|---|---|
| kernel crossings | `Close`, `Reify`, `QuestionOf`, `InterpretContent`, `InterpretAct`, `Singleton` |
| force/performance | `Assert`, `Ask`, `Command`, `Express`, `Mention`, `Vocative`, `Perform`, `PerformUtterance`, `Do` |
| dynamic effects/indexicals | `Presuppose`, `Supplement`, `Refer`, `Context`, `Typical`, `Stereotypical`, `Witnesses`, `Deictic` |
| logic/intensional control | `¬`, `∧`, `∨`, `→`, `↔`, `⊕`, `∀`, `∃`, `Joi` |
| reference/plural | `Among`, `Combine` |
| relation formers | `DropPlace`, `Tanru`, `Scalar` |
| abstractions | `Measure`, `TruthValue`, `ExperienceOf`, `ProcessOf`, `ActivityOf`, `Concept`, `Abstract` |
| questions | `Polar`, `OpenQ`, `Answer`, `PolarAnswer`, `TupleAnswer`, `ContextualAnswer`, `UnresolvedAnswer` |
| collections/math | `Set`, `SetOf`, `List`, `Tuple`, `Card`, `Interval`, `ZipWith`, `=`, `<`, `≤`, `+`, `−`, `×`, `÷`, `∈` |
| signs | `OpaqueQuote`, `StructuredQuote`, `NameSign`, `SentenceSign` |
| discourse | `NewTopic`, `Resume`, `PriorDiscourse`, `FollowingDiscourse` |
| utterance/sign/metadata facts | `Realizes`, `SpeakerOf`, `AudienceOf`, `LocutionOf`, `DeicticTimeOf`, `DeicticPlaceOf`, `TextOf`, `Denotes`, `Quotes`, `Utters`, `Label` |
| packaging | `Smusni`, `Words`, `Word` |

Every primitive not already given a more specific signature above has the
following version-0 signature or signature schema:

```text
Close       : PredTerm<ρ> -> Content when ρ is closeable
Singleton   : T -> Referents<T>
Reify       : Content -> Proposition
QuestionOf  : Query<A> -> Question
InterpretContent : Sign<K>|SignToken<K> -> Content
InterpretAct     : Sign<K>|SignToken<K> -> Act<F>
                   where F is graph-known or fixed by the expected type

Presuppose : Content × Content -> Content
Supplement : Content × Content -> Content
Context     : dependency* -> RefComp<Referents<T>>
Refer       : Property<Referents<T>> -> RefComp<Referents<T>>

Perform          : Act<F> -> Discourse
PerformUtterance : TranscriptEntry -> Discourse
Do               : Performable^n -> Discourse, n >= 2
NewTopic, Resume : Discourse -> Discourse
PriorDiscourse, FollowingDiscourse : Discourse

Tanru  : PredTerm<μ> × PredTerm<ρ> -> PredTerm<ρ>
Scalar : ScalarKind × PredTerm<ρ> -> PredTerm<ρ>
DropPlace : (r : PredTerm<ρ>) × (positive integer literal p in ρ)
            -> PredTerm<ρ-p>

Joi : Content^n -> Content, n >= 2
    | Discourse^n -> Discourse, n >= 2

Polar     : Content -> Query<()>
OpenQ     : Fn<(A1 ... An), Content> -> Query<(A1 ... An)>, n >= 1
Answer    : Query<A> × AnswerSelection<A> -> Content
PolarAnswer      : AnswerPolarity -> AnswerSelection<()>
TupleAnswer      : Tuple<(A1 ... An)> × [AnswerExhaustivity]
                   -> AnswerSelection<(A1 ... An)>
ContextualAnswer : [AnswerExhaustivity] -> AnswerSelection<(A1 ... An)>
UnresolvedAnswer : AnswerSelection<(A1 ... An)>

The bracketed `AnswerExhaustivity` operand is optional per section 12.2:
omission is the canonical spelling of undetermined exhaustivity, not a
defaulted value. Zero-operand `ContextualAnswer` prints as the bare atom.

Set     : T* -> Set<T>
SetOf   : PureProperty<T> -> Set<T>
List    : T* -> List<T>
Tuple   : A1 × ... × An -> Tuple<(A1 ... An)>
Card    : Set<T>|List<T> -> Cardinal
Interval: T × T × EndpointInclusion × EndpointInclusion -> Interval<T>
          for a registered ordered T
ZipWith : Fn<(A1 ... An), Content> × List<A1> ... × List<An> -> Content
=       : T × T -> Content
<, ≤    : T × T -> Content for a registered ordered T
+, −, ÷ : Number × Number -> Number
×       : Number × Number -> Number
        | Set<A> × Set<B> -> Set<Tuple<(A B)>>
∈       : T × Set<T> -> Content

Realizes       : UtteranceToken × Act<F> -> Content
SpeakerOf      : UtteranceToken × Referents<Entity> -> Content
AudienceOf     : UtteranceToken × Referents<Entity> -> Content
LocutionOf     : UtteranceToken × Referents<Locution> -> Content
DeicticTimeOf  : UtteranceToken × Referents<Eventuality> -> Content
DeicticPlaceOf : UtteranceToken × Referents<Location> -> Content
TextOf         : UtteranceToken|SignToken<K> × Text -> Content
Denotes        : SignToken<K> × A -> Content
Quotes         : SignToken<K> × Quotable -> Content
Utters         : Referents<Entity> × Utterable -> Content
Label          : LabelLevel × Number × A -> Content
```

`InterpretAct` has no force-erased result. If neither the graph nor the expected
position determines `F`, projection fails with a registered reason. `Quotable`
abbreviates the closed overload set `Text`, `Sign<K>`,
`SignToken<K>`, `TranscriptEntry`, and `Discourse`. `Utterable` abbreviates
`Act<F>`, `TranscriptEntry`, `Sign<K>`, and `SignToken<K>`. These two names are
signature metanotation, not type atoms or printable union wrappers. Likewise,
the vertical bars above denote closed overloads, not a surface union type.

In the `Context` schema, `T` is fixed by the expected operand/result type and
every printed dependency is an in-scope value named by the graph's dependency
set. With zero dependencies the canonical surface form is the bare atom
`Context`, not `(Context)`.

`Tanru` preserves the head row `ρ`. It makes no generic claim that modifier and
head share x1: any more specific argument link must come from the represented
tanru reduction and its typed registry evidence. Arithmetic
is exact and partial where ordinary mathematics is partial, such as division
by zero; an invalid represented operation is rejected with a registered
invalid-graph reason. `Label` is an
ordinary analyzer fact about its target, not a wrapper which changes the
target's value or force.

`PriorDiscourse` and `FollowingDiscourse` are inert graph-owned references to
the immediately preceding or following represented discourse segment. They do
not perform that segment, repeat its effects, or contribute content by
themselves. Despite their `Discourse` value type, they are reference-only
constants: they are legal only as ordinary values in registered fact operands
such as `Quotes`, `Denotes`, or another relation whose signature accepts the
value. This restriction follows the value through `Let` and every other
identity-preserving binding; rebinding cannot turn it into a performable
computation. They are illegal as the top-level `Smusni` performable and as
`Perform`, `Do`, `Joi`, `NewTopic`, or `Resume` operands. If the graph requires
the referenced segment to be performed at that site, the smallest affected
performable is rejected with a registered invalid-graph reason instead of
treating the reference as the segment's computation.

`Smusni`, `Words`, and `Word` have the packaging grammar in section 2.4. No
primitive obtains an additional overload merely because an input-model field
would fit it.

The derived prelude is a second closed registry. A monomorphic prelude value is
semantically equivalent to nesting its listed `Let` binding outside the
document body in dependency order. A type-parameterized schema instead
elaborates each occurrence to a fresh monomorphic local `Let` selected by its
operand and expected types; version 0 does not add System-F polymorphism or
install several same-named global bindings. Canonical output omits either kind
of wrapper and prints the registered prelude name or glyph directly. Such a
name has no semantics beyond its displayed definition, cannot be shadowed by a
document binding, and MAY always be expanded without changing type, evaluation
order, effects, or graph sharing.

| Group | Prelude names |
|---|---|
| description/naming | `DescribedAs`, `Named` |
| modality | `InnatelyCapable`, `MotionVector` |
| generalized quantification | `Some`, `No`, `Every`, `Exactly`, `AtLeast`, `AtMost`, `MoreThan`, `FewerThan` |
| mathematical | `≠`, `>`, `≥`, `∪`, `∩` |
| deictic defaults | `This`, `That`, `Yonder` |

A type-parameterized equation `Name args = rhs` elsewhere in this specification
denotes the corresponding fresh monomorphic prelude binding
`(Let ((Name signature (λ (args) rhs))) ⟦body⟧)`, with nested lambdas where the
signature is curried. An equation without arguments binds its right-hand value
directly. `⟦body⟧` in the displayed definitions below is specification
metanotation for the enclosed document body and is not a datum token. This is
core-language `Let`, not an implementation macro with additional evaluation
rules.

Version 0 currently includes this definition:

```lisp
(Let ((DescribedAs
       (Fn ((Referents Entity)
            (Referents Entity)
            (Fn ((Referents Entity)) Content))
           Content)
       (λ (($describer (Referents Entity))
           ($described (Referents Entity))
           ($property
             (Fn ((Referents Entity)) Content)))
         ((DropPlace skicu 3)
           $describer
           $described
           $property))))
  ⟦body⟧)
```

Deleting x3 and filling surviving x1, x2, and x4 produces a predicate term
whose only remaining slot is its local event; the declared `Content` result
inserts ordinary closure. The normative deletion evidence is the version-0
`voi` reduction's explicit three-role contract; its registered evidence id is
`smusni.voi.no-audience`. The implementation MUST verify both that contract and
the `skicu` row used by this definition against the checked-in registry. A
mismatch makes the prelude definition unavailable and projection fails with a
registered reason; it does not silently change the definition.

The standard name-description helper is likewise reducible:

```lisp
(Let ((Named
       (Fn (Text (Referents Entity)) Content)
       (λ (($name Text) ($named (Referents Entity)))
         (cmene
           (NameSign $name)
           $named
           Speaker))))
  ⟦body⟧)
```

The current speaker is the live `cmene` x3 name-user in the version-0 `la`
reduction; it is not an omitted place and is not deleted. This follows the
speaker-relative naming expansion rather than treating namehood as an
unqualified binary relation. Another graph-represented name-user fills x3
explicitly through its own registered naming relation.
After x1, x2, and x3 are filled, expected `Content` insertion closes only the
local event slot when the verified `cmene` row licenses one.
The implementation MUST verify the `cmene` row and the speaker-relative `la`
reduction together; neither English gloss nor operand count substitutes for
that registered evidence.

Innate capability is also a lexical composition. `InnatelyCapable` receives
the actual bearer and a property which can reconstruct the tagged host for a
candidate bearer and possible event:

```lisp
(Let ((InnatelyCapable
       (Fn ((Referents Entity)
            (Fn ((Referents Entity) (Referents Eventuality)) Content))
           Content)
       (λ (($bearer (Referents Entity))
           ($host
             (Fn ((Referents Entity) (Referents Eventuality)) Content)))
         (jinzi
           (λ (($candidate (Referents Entity)))
             (kakne
               $candidate
               (Bind (($possible (Referents Eventuality)
                       (Refer
                         (λ (($event (Referents Eventuality)))
                           ($host $candidate $event)))))
                 $possible)))
           $bearer))))
  ⟦body⟧)
```

The reusable axial-motion helper removes only the route place that its
displacement interface does not represent:

```lisp
(Let ((MotionVector
       (Fn ((Referents Eventuality)
            (Referents Entity)
            (Fn ((Referents Entity) (Referents Entity)) Content))
           Content)
       (λ (($motion (Referents Eventuality))
           ($mover (Referents Entity))
           ($displacement
             (Fn ((Referents Entity) (Referents Entity)) Content)))
         (∃
           (λ (($destination (Referents Entity))
               ($origin (Referents Entity)))
             (∧
               ((DropPlace muvdu 4)
                 $mover
                 $destination
                 $origin
                 :Eventuality $motion)
               ($displacement $destination $origin)))))))
  ⟦body⟧)
```

Deleting x4 is part of this helper's declared meaning, not a general rule that
an omitted Lojban place means `zi'o`. A reduction with a represented route must
use `muvdu` with x4 intact or a different transparent helper. Likewise this
helper cannot be called until its mover and motion-event policy are known. Its
registered deletion evidence id is `smusni.motion-vector.no-route`; the
implementation MUST verify the `muvdu` row and this reduction contract
together.

The missing x3 of `kakne` is the ordinary distinct contextual/defaulted
condition place supplied by expected `Content` closure; it is not innateness
and is not deleted. CLL's `ka'e` discussion explicitly contemplates possible
enabling conditions, so the role remains semantically live. `jinzi` says that
the entire capability property is inherent in the bearer. For a host whose x1
is `$bearer`, a `ka'e` tag supplies a `$host` lambda that substitutes its first
parameter for x1, its second for `:Eventuality`, and preserves every other
argument and relation former. This schema therefore works under quantifiers
and negation without asserting that `$possible` actually occurs. A missing,
deleted, or nonrecoverable host x1 makes this reduction unavailable rather than
causing the renderer to guess a bearer.

The possible-event `Bind` is inside `kakne` x2, whose verified scope-policy row
MUST be `Intensional`; consequently the host property is part of the capability
content and is not asserted as an actual event. Until both the `kakne` lexical
row, the `jinzi` row, and that policy row are verified, this helper is
unavailable and `ka'e` makes projection fail with a registered reason. No `DropPlace kakne 3` is licensed
by mere absence of a source condition operand.

The deictic defaults are inert value bindings:

```lisp
(Let ((This (Referents Entity)
       (Deictic Proximal CurrentGround)))
  (Let ((That (Referents Entity)
         (Deictic Medial CurrentGround)))
    (Let ((Yonder (Referents Entity)
           (Deictic Distal CurrentGround)))
      ⟦body⟧)))
```

The type namespace is independently closed. Its atomic members are the sorts
listed in section 3.1 plus `Content`, `Discourse`, `TranscriptEntry`,
`Performable`, `UtteranceToken`, and the force/sign/literal families. Its type
constructors are:

```text
PredTerm Row Fn Referents RefComp Act Query AnswerSelection GQ
Set Group List Tuple Interval Sign SignToken PlaceOf
```

Type position is determined by the grammar and registered signatures, so a
spelling such as `Concept` may be both a type atom and a callable crossing
without ambiguity. In a collection, a first child recognized as a type is the
explicit element type; a first-class callable of the same spelling therefore
requires that explicit element type before it. `Open` inside `Row` is the closed
unknown-tail marker, not a callable value.

`Let`, `Bind`, `LetRec`, `λ`, `Utterance`, `Sign`, `At`, place keywords, and
collection element-type operands are grammar forms, not callable intrinsics.

Closed literal families include force, sign kind, answer exhaustivity, answer
polarity, scalar kind, label level, endpoint inclusion, scale, and lexical scope
policy. A literal is valid only in a signature position that names its family;
the same spelling MAY occur in two disjoint families.

```text
Force              = Assertion | Question | Directive | Expressive
                     | Mentioning | Address
SignKind           = Name | Sentence | Quotation | Word | Letteral
                     | MathExpression | Connective | Text | Structured | Opaque
AnswerPolarity     = Yes | No | Unknown
AnswerExhaustivity = Exhaustive | MentionSome
ScalarKind         = OtherThan | Opposite | Neutral
LabelLevel         = Item | Division
EndpointInclusion  = Open | Closed
RowTailMarker      = Open
Proximity          = Proximal | Medial | Distal
LexicalScopePolicy = Extensional | Intensional | Opaque
```

Scale literals are the finite members of the versioned generated scale table;
that table declares the type and source of each member. An unknown scale does
not become a new PascalCase literal.

The closed irreducible deictic constants are `Speaker`, `Audience`, `Now`,
`Here`, and `CurrentGround`; `This`, `That`, and `Yonder` are the prelude values
above. `PriorDiscourse` and `FollowingDiscourse` are graph-owned discourse
constants. Scale literals such as `DistanceScale` occur only in a scale-typed
position. Registered indicator relations such as `Contrast` and generated event
relations belong to their versioned generated tables. Event facets use the
exact lexical or transparent-prelude reductions required by section 10.4; an
opaque `LongDuration`-style constructor is not minted from a model enum.

### 14.2 Lexical and generated registries

Lowercase roots obtain their row, place types, event licensing, defaultability,
and normalized identity from the versioned semantic dictionary. Dynamic lexical
places additionally require a generated policy row with this schema:

```text
normalized-root, original-ordinal, scope-policy, evidence-id
```

Tag reductions, event facets, model indicator relations, quantity scales, and
other generated families have the same closure requirement: the implementation
ships a versioned source table, validates it without network access, and fails
closed on missing or contradictory rows. A tag/event row contains a complete
typed lexical or prelude expansion plus its evidence. Generated PascalCase
members are permitted only for a separately declared irreducible family or a
transparent prelude definition; a table cannot mint an opaque semantic
predicate from a source spelling.

Every `DropPlace` occurring in a prelude or generated expansion additionally
requires a deletion-evidence row. The row identifies the exact owner and
lexical ordinal and states why that semantic role is absent; it cannot cite
surface omission as its reason.

Format version `0` is minted when the registry tables below are reviewed for
semantic correctness, checked in, and this document stops calling itself a
candidate. Before that mint, this document is a design and implementation
candidate: its kernel and transparent definitions may be implemented, but no
renderer output is yet conformant normal form.

Minting is a review decision, not a cryptographic ceremony. There is no
manifest, no bundle digest, no generator identity derived from a digest
closure, and no byte-mirrored copy of the generator's own sources. Code review
is the semantic authorization for adding or changing a row, and ordinary git
history is the version history. What the format versions is stated precisely:

- the `0` in `(Smusni 0 ...)` versions the public grammar, the static typing,
  the kernel semantics, and the stable meanings of the normal forms;
- reviewed lexical and reduction coverage is checked-in data versioned by
  ordinary git. It may grow within version `0` whenever a new row instantiates
  an existing rule without changing that rule's meaning;
- a backward-incompatible grammar, type, kernel, or normal-form semantic change
  requires a new format version; and
- a row correction is a reviewed change, not a new mint. If a correction changes
  semantics an implementation previously emitted, it is recorded prominently in
  release notes and its compatibility is decided by its actual consumer impact.

The generated tables are ordinary deterministic build output:

- both the authored source tables and the generated runtime tables are checked
  in;
- the generator has a deterministic order and a canonical serialization
  adequate for clean diffs, and two clean runs produce byte-identical output;
- `generate --check` fails when the checked-in output differs from a fresh run;
  and
- primary keys are unique, every referenced id resolves, closed enums match the
  generated enums, templates parse and typecheck, expansion dependencies are
  acyclic, and the semantic-coordinate scanner has exact coverage.

Generated tables are JSON Lines: each row is one NFC-normalized object
serialized by RFC 8785 JSON Canonicalization Scheme (JCS), followed by LF. Rows
sort by their declared primary-key tuple: strings by Unicode scalar-value
order, integers numerically, and composite keys left to right. Types,
signatures, rows, and expansion templates embedded in a field use the canonical
smusni spelling from this document. NFC and stable JSONL ordering are retained
because they produce readable deterministic diffs, not because they form an
evidence chain. Consumers may compile these rows to another internal
representation.

The normative generated artifacts have these logical schemas:

```text
EvidenceRow =
  evidence-id, exact-locator, adjudication-note

SlotRow =
  label, accepted-type-schema, close-policy, lexical-provenance,
  evidence-id

ClosePolicy = Required | Contextual | LocalExistential

LexicalRow =
  root, normalized-root, word-class, dictionary-source-id,
  dictionary-entry-id, ordered-numbered-slot-rows, optional-event-slot-row

ScopePolicyRow =
  normalized-root, original-ordinal, scope-policy, evidence-id

PlaceDeletionEvidenceRow =
  expansion-owner, normalized-root, original-ordinal,
  input-row-schema, result-row-schema, surviving-slot-map,
  semantic-absence-contract, evidence-id

TagReductionRow =
  source-family, source-member, applicability-guard, operand-types,
  source-place-map, host-event-map, required-graph-identities,
  typed-expansion-template, resulting-type-schema, evidence-id

RelationFormerReductionRow =
  former-kind, source-owner, applicability-guard, operand-row-schemas,
  result-row-schema, total-provenance-map,
  typed-link-or-expansion-contract, evidence-id

GeneratedRelationRow =
  family, PascalCase-name, type-parameters, complete-signature-schema,
  context-effect-summary, stability-summary,
  irreducibility-reason, evidence-id

ScaleLiteralRow =
  PascalCase-name, raw-value-type, source-members, evidence-id

SemanticCoordinate = {
  category, owner, kind, member, optional-branch
}

SourceOrigin = {
  source-artifact, module-path, type, member-locator
}

CoordinateOrigin = {
  semantic-coordinate, projection-id,
  nonempty-source-origins: SourceOrigin+
}

SemanticTypeRow =
  semantic-type-id, raw-model-type, class, reachability-kind,
  stable-member-map, evidence-id

StableMember = {
  source-member-locator, stable-semantic-member
}

ProjectionIrTypeRow =
  projection-ir-type-id, structural-schema, evidence-id

ProjectionIrSchema =
    Unit
  | Product { fields: IrField+ }
  | Sum { variants: IrVariant+ }
  | Sequence { element-schema }
  | Map { key-schema, value-schema }
  | Optional { element-schema }
  | RawModelRef { raw-model-type }
  | SmusniDatum { v0-type-schema }

IrField = { stable-name, schema }
IrVariant = { stable-name, payload-schema }

ContextValueRow =
  context-value-id, context-value-kind, structural-schema, evidence-id

ProjectionDeclarationRow =
  projection-id, semantic-type-id,
  coordinate-origins: CoordinateOrigin*,
  serialization-shape, evidence-id

CheckedExclusionRow =
  source-origin, exclusion-kind, exact-reason, evidence-id

AlgorithmFailureSiteRow =
  site-id, phase, algorithm-id, owner-input-name,
  allowed-failure-site-kind, evidence-id

DispositionOwner =
    Semantic { semantic-coordinate }
  | AlgorithmFailure { site-id }

ProjectionInputSlot = {
  name, domain
}

ProjectionInputDomain =
    RawOwner { raw-model-type }
  | ModelValue { raw-model-type }
  | ProjectionIr { projection-ir-type-id }
  | SmusniDatum { v0-type-schema }
  | GraphIdentity { raw-model-type }
  | ContextValue { context-value-id }

OccurrencePath =
    CurrentOwner
  | GraphRoot
  | Member { base-path, semantic-coordinate }
  | Parent { base-path, raw-model-type }
  | Dereference { base-path, raw-model-type }

OccurrenceView =
    RawOwner
  | ModelValue
  | LoweredValue
  | GraphIdentity

AlgorithmUse = {
  algorithm-id, type-arguments, input-bindings
}

ProjectionValueSource =
    Occurrence { occurrence-path, occurrence-view }
  | AlgorithmResult { algorithm-use }
  | ContextValue { context-value-id }
  | Constant { v0-type-schema, datum }

InputBinding = {
  input-name, projection-value-source
}

FactUse = {
  fact-id, type-arguments, input-bindings
}

TargetUse = {
  target-contract-id, type-arguments, input-bindings
}

FactRequirement = {
  fact-use, required-value
}

Decision =
    Outcome { Target target-use | Failure reason-id }
  | If { fact-use, then-decision, else-decision }

ProjectionFailureSite =
    TypedPosition { expected-v0-type-schema, minimum-raw-owner-type }
  | WholeGraph { raw-root-type }

FailureClass = InvalidGraph | RouteUnavailable | TrackedSpecGap
             | ImplementationInvariant

ProjectionFailureReasonRow =
  reason-id, owner, failure-site, failure-class, evidence-id

ProjectionFactRow =
  fact-id, type-parameters, input-slots, evidence-id

ProjectionAlgorithmRow =
  algorithm-id, type-parameters, input-slots, result-domain,
  required-facts, failure-sites, context-effect-summary,
  stability-summary, evidence-id

ProjectionResultDomain =
    ProjectionIr { projection-ir-type-id }
  | SmusniDatum { v0-type-schema }

TargetContractRow =
  target-contract-id, type-parameters, input-slots, result-domain,
  required-facts, implementation, evidence-id

TargetResultDomain =
    SmusniDatum { v0-type-schema }
  | NoSurfaceDatum

DispositionRow =
  owner, disposition, total-decision, evidence-id

PreludeRow =
  name, type-parameters, complete-signature-schema, canonical-definition,
  direct-dependencies
```

The primary keys for the top-level tables, in schema order and excluding the
inline tagged and slot types above, are `evidence-id`,
`normalized-root`, `(normalized-root, original-ordinal)`,
`(expansion-owner, normalized-root, original-ordinal)`,
`(source-family, source-member, applicability-guard)`,
`(former-kind, source-owner, applicability-guard)`,
`(family, PascalCase-name)`, `PascalCase-name`, `semantic-type-id`,
`projection-ir-type-id`, `context-value-id`, `projection-id`,
`(source-artifact, module-path, type, member-locator)`, `site-id`,
`reason-id`, `fact-id`, `algorithm-id`, `target-contract-id`, `owner`, and
`name`.
Duplicate primary keys are invalid. The generic
`Tanru` and `Scalar` row-preservation rules in section 4.6 need no per-use row;
only a more specific argument link, scale, or source reduction uses a
`RelationFormerReductionRow`.

Independently of table primary keys, every `GeneratedRelationRow.PascalCase-name`
is globally unique across generated-relation families and disjoint from every
primitive, transparent-prelude, generated constant, scale, and literal spelling
in value position. Generated relations do not overload and never use the
nonprinting `family` field to resolve one surface atom. This does not prohibit
the explicitly enumerated closed primitive overloads in section 14.1. The
independently closed type namespace may still reuse a spelling in a
syntactically determined type position as specified in section 14.1.

`SlotRow.label` is a positive original numbered label or `Eventuality`.
`Contextual` licenses ordinary contextual closure but does not decide the
graph-specific `Fixed`/`Underspecified` dependence classification;
`LocalExistential` is valid only for an event slot. `lexical-provenance` is the
normalized root plus original dictionary ordinal for numbered slots and the
registered event license for `Eventuality`. Thus place type, defaultability,
event licensing, and original identity are all machine-readable rather than
inferred from definition prose. Every `LexicalRow` object contains the
`optional-event-slot-row` key; its value is JSON `null` when the root has no
event slot.

Every `evidence-id` is a foreign key to exactly one `EvidenceRow`. An
`EvidenceRow` is reviewed rationale: a human-readable locator a reviewer can
check and the note that records what was adjudicated. It is not a hash-attested
evidence graph, and a bare label with no row is still not evidence. Every
projection-failure reason names the same closed disposition owner as the
`DispositionRow` which selects it. A scale literal is a raw first-order value of
`raw-value-type`; for example a raw `Scale` may use the ordinary singleton lift
at an operand requiring `Referents<Scale>`. `word-class` is a curated
provenance label rather than a closed format vocabulary; its exact checked-in
values remain deterministic inputs to generation and validation.

`DispositionOwner` is a tagged value, not a free label plus a second kind
field. A semantic owner contains the scanner's exact structured coordinate. An
algorithm-failure owner contains one authored `AlgorithmFailureSiteRow`
identifying an exact semantically possible projection failure branch. The
namespaces are disjoint: algorithm failures are not fake model fields or
`Document` derived facts. The primary key ordering is the tag
`Semantic` before `AlgorithmFailure`, followed respectively by the coordinate
fields in schema order or by `site-id`. The authored rows generate closed
runtime enums; compiled enums are checked outputs rather than an independent
authority. Validation checks these independent equalities:

```text
scanned semantic coordinates
  == authored SemanticCoordinate disposition owners
  == minted SemanticCoordinate disposition owners

authored AlgorithmFailureSite rows
  == generated AlgorithmFailureSite variants
  == authored AlgorithmFailure disposition owners
  == minted AlgorithmFailure disposition owners
```

An unknown semantic coordinate is registry or executable drift and fails before
rendering; it is not a runtime projection-failure reason. Failure of an
algorithm which the implementation can make total, including planner
nontermination or non-convergence, is likewise an implementation error rather
than an `AlgorithmFailureSite`. An algorithm-failure owner has no semantic
disposition: its `DispositionRow` carries the non-semantic `Failure` marker
with exactly one unconditional failure leaf, and the failure's section-16.2
class comes from its
`ProjectionFailureReasonRow` — an invalid-graph rejection (an unbound variable,
a non-performable root) and an unavailable implementation route are both
legitimate algorithm-failure sites, and the reason row says which one this is.
A recoverable alternative is a `ProjectionFactRow` decision on its semantic
owner, not an algorithm-failure owner.

`SemanticTypeRow.class` is `Product`, `Sum`, `ScalarCodeList`, `ScalarNewtype`,
or `Alias`; a scalar code list contains only unit alternatives and an alias
names its exact expanded target schema. `reachability-kind` is
`GraphReachable` or `DesignatedAuxiliary`. `stable-member-map` assigns every source member,
including a tuple payload locator, a stable semantic member identifier. The
generated closed `RawModelType` domain comes from these rows. Internal typed
projection states use the separate generated `ProjectionIrType` domain; neither
domain is smusni section-2.2 type syntax. `ProjectionIrTypeRow` and
`ContextValueRow` use the closed recursive `ProjectionIrSchema`; their authored
IDs generate the only legal IR and context-value identifiers.

`ProjectionDeclarationRow.serialization-shape` is a closed tagged declaration
of `Structural`, `Flattened`, `Tagged`, `Untagged`, or `CollapsedScalar`. It
contains the exact constructor/payload mapping, surface and non-discriminator
keys, discriminator computation, nested flattening, and typed derived keys
applicable to that shape. The row's `coordinate-origins` is the sole authority
for the exact many-to-many mapping from source origins to emitted semantic
coordinates; the flat origin and coordinate sets are mechanically derived as
its two projections. The serialization shape and coordinate-origin mapping are
shared by the scanner and serializer and MUST agree on every member and branch.
`CheckedExclusionRow.exclusion-kind` and
`AlgorithmFailureSiteRow.phase` and `allowed-failure-site-kind` are
closed generated domains. The authored rows determine their member set.

Every `SemanticTypeRow` has exactly one `ProjectionDeclarationRow`, including
ordinary derived products, sums, aliases, and scalar newtypes. Those ordinary
shapes use a checked `Structural` declaration rather than an implicit direct
source path. A projection declaration for an `Alias` row MUST have an empty
`coordinate-origins` mapping; every non-`Alias` row MUST have a nonempty mapping.
The alias's exact expanded target schema and reachability are still scanned and
validated, but a transparent alias contributes no semantic coordinate and does
not duplicate its target's coordinates. Every entry in `coordinate-origins`
has a distinct `semantic-coordinate`, its `projection-id` equals its enclosing
row's key, and that id is a foreign key. A `semantic-coordinate` is globally
unique across the `coordinate-origins` entries of all projection declarations.
`ProjectionDeclarationRow.semantic-type-id` is also a foreign key and is
unique, and its complete value set equals the `SemanticTypeRow` primary-key set.
Specialized declarations merely choose another serialization-shape variant;
they do not create a second authority for the same type.

Fact and target decisions are typed applications, not bare identifiers. Every
input slot has one of the six disjoint domains above, and every binding names
one exact runtime source. An `OccurrencePath` is interpreted relative to the
current occurrence of the disposition owner, not merely its static coordinate.
`Member` follows the exact declared member occurrence, `Dereference` follows
the graph identity stored at that occurrence, `Parent` ascends to a unique
containing occurrence, and `GraphRoot` selects the one graph root. An ambiguous
parent, wrong raw type, missing identity, nonunique path, or inaccessible
occurrence makes the binding invalid rather than choosing heuristically.
Container members are bound as whole typed values; selection of their elements
is an explicit projection algorithm, not an implicit occurrence-path index.
The view determines whether the binding supplies the raw owner, its typed model
value, a successfully lowered smusni value, or its graph identity.

`AlgorithmResult` invokes one typed `AlgorithmUse`; context sources use an
authored closed context-value identifier; and a constant is a parsed,
typechecked v0 datum. Bindings are total by input name, with no duplicates or
extra values. Type arguments instantiate every declared type parameter exactly
once under the rigid monomorphic rules below. Occurrence paths also retain the
actual ancestor chain used when a failure is attributed to a larger owner, so
an algorithm site cannot substitute a different occurrence of the same model
type.

Every `ProjectionFactRow` declares one total, side-effect-free predicate over
its typed input slots. Its `fact-id` generates the evaluator enum variant and
has one explicit runtime match arm; compiled evaluator variants must equal fact
rows exactly and wildcard dispatch is forbidden. Facts include the
availability of an exact registered definition, sign identity for quotation,
exact question answer type, coordinate-closed termset profile, higher-order
crossing, quantity basis, event-facet reduction, and scalar-reduction
preconditions. Evaluating a fact cannot change context, emit an effect, consume
a witness, or fail. Decision-tree order is observable only as deterministic
control flow, and every execution reaches exactly one leaf.

Likewise, each authored `ProjectionAlgorithmRow.algorithm-id` generates one
executor enum variant implemented by one exhaustive runtime arm; compiled
executor variants must equal algorithm rows exactly. Its inputs occupy the
raw-model, projection-IR, smusni, graph-identity, and context domains
explicitly. An executor returns the closed result
`Success { value } | Failure { site-id }`; the success value inhabits the
declared `ProjectionResultDomain`, and the failure site must occur in that row's exact
`failure-sites` set. Every `AlgorithmFailureSiteRow` names that same algorithm
and one `RawOwner` input slot, which identifies the smallest semantic owner the
failure is attributed to. The site transfers control to its unique
algorithm-failure disposition row, whose reason and failure site are what the
structured failure channel of section 16 carries. A failure never produces a
datum, so no executor may return an undeclared site, and every declared site
must be returned by a focused fixture and have exactly one originating
algorithm.

The site's `owner-input-name` must resolve to a `RawOwner` slot of that
algorithm. Its `allowed-failure-site-kind` must equal the tag of the unique
joined `ProjectionFailureReasonRow`: for `TypedPosition`, the slot's model type
equals `minimum-raw-owner-type`; for `WholeGraph`, the slot type and reason
root are both exactly `SemanticGraph`. Thus the executor attributes the failure
to the actual occurrence selected by the registered site rather than merely to
a same-named model type.

An `AlgorithmResult` binding is the only producer of a `ProjectionIr` input.
Algorithm dependencies form a finite acyclic graph after type instantiation;
their input bindings are checked by the same rules, and an internal failure
propagates its declared site without being relabelled. A target
`ProjectionAlgorithm` may reference only an algorithm with a `SmusniDatum`
result whose v0 type equals the target row's
`TargetResultDomain::SmusniDatum { v0-type-schema }`; IR-producing
algorithms are preparation steps and cannot be target leaves directly. A
`ProjectionFactRow` may bind an `AlgorithmResult` only when the referenced
algorithm returns `ProjectionIr`, has no failure sites or required facts, has
identity context and no effects, and is site-stable. Such a fact-safe
preparation is total and pure; all other algorithm results are forbidden in
fact bindings. This permits facts over typed prepared IR without hiding a
failure or making fact evaluation effectful.

Validation follows every transitive `AlgorithmUse` chain to every reachable
target. Every failure is terminal for the render. A `TypedPosition` failure's
expected v0 type must equal the unique nearest enclosing `SmusniDatum`
position, including the target result when preparation occurs through IR-only
slots, and the site's actual occurrence must lie within that position's
dependency closure. If one reusable algorithm or site can reach incompatible
result schemas, the registry must split the algorithm or the site, or attribute
the failure to the whole graph; one fixed site cannot be reused across
incompatible callers.

The complete instantiated lowering dependency graph is acyclic. It includes
every `TargetUse`, `FactUse`, and `AlgorithmUse` binding and every
`OccurrenceView::LoweredValue` edge, not only direct algorithm calls.
Recursion in semantic graph values is represented later by validated
`Bind`/`LetRec` planning; it is never encoded as registry-schema dependency
recursion.

`required-facts` are exact `FactRequirement` applications. On every path to a
target leaf, the accumulated fact conditions must entail the target's
requirements and those of any called algorithm, and the supplied bindings and
type arguments must typecheck. A partial lookup, unavailable template input,
or unmet algorithm precondition therefore needs an explicit guarded failure
leaf rather than an arbitrary algorithm failure.

A `DispositionRow.disposition` is either one of the five semantic dispositions
of section 14.4 or the dedicated marker `Failure`. `Failure` is not a semantic
disposition and never counts toward expressive completeness: it records that
no route is taken at this owner. Every algorithm-failure owner uses it, and so
does a semantic owner whose route is a tracked spec gap until that gap is
closed; the section-16.2 class of each failure emitted under such a row comes
from its reason row's `failure-class`. The decisions obey these additional
invariants:

- `DirectLowering`, `ProvenDesugaring`, `NotationDefault`,
  `ProvenanceSuppression`, and `DiagnosticCollection` contain at least one
  target leaf. The disposition class describes the source-to-target relation,
  so the same target contract may be reused by different disposition kinds.
  Positive rows may contain guarded failure leaves where the reduction has an
  explicit precondition.
- a `Failure` disposition contains no target leaf. It normally has one
  unconditional failure leaf; a decision tree is permitted only to select
  between distinct exact failure sites.
- implementation coverage is not part of this decision. A semantic owner whose
  route this implementation lacks keeps its semantic disposition, records
  `ImplementationUnavailable` coverage, and its runtime failures resolve
  through reason rows classed `RouteUnavailable`.
- every target leaf resolves to one `TargetContractRow`; every failure leaf
  resolves to one `ProjectionFailureReasonRow` whose structured owner equals
  the disposition owner; and target, fact, reason, and owner identifiers all
  use generated closed types.
- a `NotationDefault` target uses `CanonicalDefault`, a
  `ProvenanceSuppression` target uses `RecoveredAt` or `OmitNonsemantic`, and a
  `DiagnosticCollection` target uses `DiagnosticChannel`. Direct and proven
  routes may use any implementation whose typed contract they satisfy.
- `CanonicalDefault`, `RecoveredAt`, `OmitNonsemantic`, and
  `DiagnosticChannel` have result `NoSurfaceDatum`. A typed template, registry
  expansion, or target projection algorithm has a `SmusniDatum` result.

`TargetContractRow.implementation` is one of:

```text
TypedTemplate {
  canonical-template
}
ProjectionAlgorithm {
  algorithm-id
}
RegistryExpansion {
  table, key-template, expansion-column
}
CanonicalDefault {
  canonical-value
}
RecoveredAt {
  semantic-owner, target-contract-id
}
OmitNonsemantic {
  provenance-kind
}
DiagnosticChannel {
  OrderedDiagnostics
}
```

`TypedTemplate` uses the target row's typed input-slot names as checked `Hole`
names and uses `TypeParam` as below.
`ProjectionAlgorithm` resolves through exactly one `ProjectionAlgorithmRow`;
its algorithm identifier is an exhaustively matched generated enum, and the
declared contract fixes the typed inputs/result, required facts,
context/effects, and stability. The target's inputs, result, and required facts
must equal the referenced algorithm contract after type substitution.
`RegistryExpansion` names one exact registry table, typed key template, and
schema-checked expansion column; table and column identifiers are generated
from the row schemas rather than accepted as strings. `CanonicalDefault` states
the actual canonical value, and validation proves that it reconstructs the
owning source coordinate as its declared default. `RecoveredAt` identifies the
exact semantic coordinate and target contract which retain suppressed content.
Recovered-at links are acyclic, cannot point back to the suppressed coordinate,
and terminate at a nonsuppression target whose result type contains the
recovered semantic value.
`OmitNonsemantic` applies only to a source/provenance coordinate which the
semantic authority proves has no semantic value; its `provenance-kind` is a
closed generated identifier and its target result is `NoSurfaceDatum`.
`DiagnosticChannel` names only the ordered diagnostic channel of section 16.1.
Validation rejects an unparsed target string, a family bucket, a contract which
lists several possible operations, or a source coordinate merely repeated as
its own target.

`ProjectionFailureSite::TypedPosition` requires a parsed v0 expected type and
one exact reachable model owner. For each failure leaf the validator computes
the failed source dependency closure: every unavailable value, graph identity,
and context input needed by that path. The declared owner must be the unique
lowest reachable owner that contains that closure; if there is no unique local
owner the failure is attributed to the nearest typed ancestor or to the whole
graph. Merely finding the same expected type on a descendant is neither
necessary nor sufficient. `ProjectionFailureSite::WholeGraph` requires
`raw-root-type` to be exactly `SemanticGraph` and has no expected smusni type,
because no smusni type was ever established for it. The generated runtime API
preserves the same tagged distinction.

Kernel primitive context/effect/stability behavior is fixed by sections 3, 6,
and 14.1 and cannot be overridden by the registry. A lexical predicate term is
inert while assembled; its operands and eventual `Close` determine the dynamic
summary. A transparent prelude summary is computed by expanding its
`PreludeRow`, and a tag or relation-former summary is computed from its
validated expansion or link contract. Version-0 irreducible generated
relations are ordinary inert predicates: their `context-effect-summary` is
exactly the JSON object `{"context":"identity","effects":[]}` and their
`stability-summary` is exactly the JSON string
`"site-stable-within-performance"`. A generated relation requiring any other
control or effect behavior must instead be a kernel primitive or a transparent
expansion; if it is neither, projection fails with a registered reason. Missing
or contradictory summaries fail closed.
This supplies the declarations consumed by the structural purity algorithm
rather than leaving "registered pure" as an implementation choice.

An applicability guard, expansion template, link contract, or typed target
template is canonical smusni syntax extended by the registry metaform
`(Hole "name" type)`. A `PreludeRow`, `GeneratedRelationRow`,
`ProjectionFactRow`, `ProjectionAlgorithmRow`, or `TargetContractRow`
additionally admits the registry-only type metaform `(TypeParam "name")` in a
type position when that row declares the parameter. A
`PreludeRow.complete-signature-schema` alone also admits
`(PureProperty type)` recursively in any function-parameter type position.
This is a refinement schema whose structural erasure is
`(Fn (type) Content)` and whose argument must satisfy the exact purity judgment
in section 3.2; it is not the surface atom `PureProperty` and can never be
emitted. For example, the first argument of polymorphic `Every` is encoded as
`(PureProperty (TypeParam "T"))`, while a cardinal helper's returned GQ schema
contains `(Fn ((PureProperty (TypeParam "T"))) Content)` so that the future
nuclear-scope precondition survives partial application and binding.

Hole names are unique lowercase ASCII identifiers within one row, their types
use section 2.2 syntax, and every substitution MUST typecheck at the declared
hole type using only the implicit conversions permitted by section 3.1. No
registry-specific coercion or type inference is permitted. The resulting
ordinary smusni datum is then validated. `Hole` is not a surface atom and can
never be emitted. Templates use holes for the host, event, explicit operands,
anchors, and graph identities. A template may contain lowercase roots, kernel
primitives, or transparent prelude calls only.

`type-parameters` is the ordered list of type-parameter names declared by a
`PreludeRow`, `GeneratedRelationRow`, `ProjectionFactRow`,
`ProjectionAlgorithmRow`, or `TargetContractRow`. Each
name is unique ASCII and every `(TypeParam "name")` in that row MUST resolve to
it. During summary and type derivation the parameters are
rigid abstract types: distinct declared names do not unify, while repeated uses
of one name must receive one consistent monomorphic substitution. Each prelude
generated-relation, or projection use is then elaborated at its operand and
expected types as described in section 14.1. `TypeParam` is never a surface
atom and can never be emitted.
`Hole` is illegal in a `PreludeRow` and `GeneratedRelationRow` and names only a
typed template's declared smusni input slots. `TypeParam` is illegal outside
the five row families which explicitly declare type parameters. Every declared
prelude, generated-relation, target fact, target, or algorithm type parameter
must be used, and each application receives the same fresh, rigid, monomorphic
substitution rules as a prelude application.

Validation expands every template, derives its result and dynamic summary,
checks every lexical row, generated-relation signature, and place map, and
rejects a recursive prelude dependency, an unregistered atom, or a declared
result which differs from the derived one. Every `PreludeRow` is generated
mechanically from the definition
in this document. Its `complete-signature-schema` and `canonical-definition`
are respectively the displayed signature and initializer value datum of the
prelude definition, excluding the surrounding binding and `⟦body⟧`
metanotation. In a type-parameterized definition, each declared schematic type
name in a type position is mechanically replaced by the corresponding
`(TypeParam "name")`. The defined aliases `Property<T>` and `GQ<T>` are
mechanically expanded to their section-3.2 structural `Fn` types, while
`PureProperty<T>` becomes the registry refinement
`(PureProperty (TypeParam "T"))` wherever it occurs, including recursively in a
returned function's parameter list; no other rewriting is allowed. Validation
recursively erases every such refinement for structural typechecking,
independently proves its section-3.2 judgment at every application, and rejects
a refinement not justified by the canonical definition's expanded use of the
parameter. The result is NFC-normalized canonical registry spelling embedded
as a JCS string. The declared ordered `type-parameters`, signature, and
definition MUST all match the mechanically extracted definition exactly; that
rederivation is the check, and no separate digest of it is stored. An
irreducibility reason is
mandatory for every `GeneratedRelationRow`; if the same meaning has an exact
lexical/compositional reduction, the generated relation is invalid and the
reduction must be used instead.

Every emitted projection-failure reason id resolves through exactly one
`ProjectionFailureReasonRow`. A `TypedPosition` row fixes the expected type
schema and the smallest model owner at which the failure is sound; a
`WholeGraph` row fixes the root as `SemanticGraph` and contains no expected
smusni type. The row's `failure-class` fixes the section-16.2 classification of
every failure emitted under that reason, so a report's class breakdown is
registry data rather than a per-site judgment call. Two implementations
therefore cannot use the same id for different failure sites, different
classes, or invent ad hoc diagnostic ids.

The lexical place maps in `samples.md` are pedagogical candidate applications
of these schemas, not an undeclared second registry. Until the tables are
checked in and reviewed, a sample demonstrates notation shape only. Building
the initial registry is therefore the first implementation gate after this
design candidate converges. An implementation may emit a sample's normal form
only after every row it uses has been verified from the checked-in tables, and
otherwise projection fails with a registered reason. A dictionary conflict is
resolved in favor of the verified checked-in data, never by preserving the
sample.

### 14.3 Required desugarings and forbidden record shapes

| Input-model family | Normal form |
|---|---|
| predication/selbri records | `PredTerm` application and fills |
| asserted predication mode | explicit `Assert` at the owning force boundary |
| inert predication mode | predicate term/content value without force |
| restrictive predication mode | reference property or quantifier restriction |
| incidental predication mode | `Supplement` at the represented anchor |
| displayed/performative mode | displayed-content relation and explicit act/performance structure |
| definitional mode | registered logical/property definition when exact; otherwise projection failure |
| modal/tag record | modal predicate plus `Joi` |
| place conversion | base-root remapping or eta-expanded `λ` |
| relative-clause record | reference property, predicate, connector, `Supplement`, or graph-licensed subreference selection through `Among` and `Refer` |
| description/gadri record | `Refer`/typed reference operation and a property |
| quantifier record | higher-order `GQ`, lambda-taking `∀`/`∃`, or the explicit simultaneous-set reduction |
| event-property record | predicates of one event variable |
| abstraction record | `λ` or an explicit level crossing |
| utterance force | section 7.1 force disposition; no `Quote`, `Parenthetical`, or `Subordinated` constructor |
| utterance metadata record | token binder plus facts |
| sign/quotation record | typed sign value or token facts |
| sequence record | `Do` and actual discourse operators |
| exact cardinal quantity | generalized-quantifier/set/cardinality reduction |
| unsupported non-cardinal quantity | projection failure at the smallest failing owner |
| warning nodes | separate diagnostic channel |

The following do not occur in normal output:

```text
Modal Relative Lo Le La Se Te Ve Xe Import ProjectiveRecord TargetFocus
WithWarnings Warning Warnings SelectionSource OpenPredTerm Interpretable
QuantifiedContent PolyQuant Restrict EventOf AchievementOf StateOf GroupOf
Degree Phase Parenthetical Subordinated Quote
```

Source `Every` import is carried by the `Every` generalized quantifier itself;
there is no `Projective` literal or separate import node in version 0.

### 14.4 Semantic dispositions and implementation coverage

Two independent facts are recorded about every semantic coordinate, and they
must not be conflated. Its **semantic disposition** says which route this
format specifies for it, and is normative. Its **implementation coverage** says
whether one particular renderer has that route working, and is not. Every
reachable semantic source origin has exactly the coverage declared by one
checked projection rule; one source origin may project to several coordinates
only when that rule lists the complete finite fan-out.

The closed semantic dispositions are:

1. `DirectLowering`: one normal-form value directly represents it;
2. `ProvenDesugaring`: it is recoverable from an exact composition or identity;
3. `NotationDefault`: it is recoverable from a declared canonical default;
4. `ProvenanceSuppression`: it is nonsemantic provenance with a declared reason;
5. `DiagnosticCollection`: it is emitted only through the diagnostic channel.

Every semantically valid reachable coordinate needs one of these five real
routes before this format can claim the expressive completeness of section 1's
goal 4. There is no sixth disposition for “an error is emitted instead”: a
projection failure is a renderer result, not a semantic distinction expressed
in smusni primitives, so it cannot close a conformance decision. (The ledger
spells the absence of a route with the non-semantic `Failure` marker defined
in section 14.2; that marker closes nothing and counts toward nothing.)

The closed implementation-coverage values are:

- `Implemented`: this renderer projects the coordinate by its declared
  semantic route; or
- `ImplementationUnavailable`: this renderer does not, and every graph reaching
  the coordinate takes the section-16 failure route with a registered
  `ProjectionFailureReasonRow`.

Coverage is allowed to make a particular renderer incomplete and fallible. It is
never a smusni normal form, and it never licenses emitting an opaque value of
the expected type.

Permanently invalid graph states are classified separately from a valid but
not-yet-implemented route. An unbound variable, an impossible effect handler,
and model/executable drift are not renderer backlog, and a corpus report which
merges them with backlog cannot distinguish the two. Reported failures
therefore separate:

1. a specified route whose implementation is missing;
2. a coordinate with no specified route at all — a **tracked spec gap**, which
   is a language-design backlog item and does not count toward expressive
   completeness;
3. a semantically invalid graph or an impossible request; and
4. an implementation invariant failure.

Only category 1 is evidence that the design is complete and the work is
mechanical.

The initial jbotci registry maintains an exhaustive generated disposition
ledger.
Its semantic-coordinate scanner starts at the semantic model root module and
recursively resolves the complete production Rust module closure, including
inline, ordinary file, directory, and explicit-path modules. Known test,
benchmark, and example-only module bodies, including `cfg(test)`, are excluded
and recorded as such. A configuration gate that can add, remove, or alter a
production semantic declaration is unsupported unless every supported
production configuration yields the same declared surface. An unresolved
module or configuration-dependent production surface is an error. Every file
in the resolved closure is a generator input; generated output is never a
generator input. This Rust/source machinery is the conformance mechanism for
the initial jbotci registry, not a requirement on independent smusni consumers;
the checked-in typed rows are the portable authority.

All local product, sum, alias, and newtype declarations transitively reachable
from `SemanticGraph` are scanned irrespective of Rust visibility. Reachability
follows named fields, tuple payloads, aliases, and container element, key, and
value types, including private fields. Otherwise-unreachable public types must
be exactly one of:

1. an explicitly designated semantic auxiliary used to derive a successful
   graph fact, such as event-binding scope; or
2. one member of the exact checked exclusion set, with a typed reason and a
   specific explanation.

Changing a reachable type from public to private therefore cannot erase its
coordinates. Public visibility or a `Serialize` implementation alone does not
establish reachability. A new unreachable public helper fails the scan until it
is removed, made private, designated, or explicitly excluded. Read-only
traversal projections and graph-construction error values are excluded;
successful semantic dispatch values are not.

A `SemanticCoordinate` contains only category, owner, kind, member, and an
optional branch qualifier. Categories are `Document`, `Object`, `ValueStruct`,
and `Enum`. Kinds are `Constructor`, `EnumVariant`, `Field`, `VariantField`,
`Discriminator`, and `DerivedFact`. Source location is deliberately absent
from coordinate identity. A `SourceOrigin` contains source artifact, module
path, Rust type, and member locator. A tuple index such as `#0` is a source
member locator only; `SemanticTypeRow.stable-member-map` or the applicable
projection declaration assigns its stable semantic member identifier.

An enum is classified once as an algebraic sum/dispatch enum or as a scalar
code list. Every alternative of the former is a `Constructor`; every
alternative of the latter is an `EnumVariant`, and a scalar code list may
contain only unit alternatives. Payload shape never changes a constructor's
coordinate kind. Named and tuple payloads retain their stable semantic member
identities even when serde is untagged or collapses the enclosing value to a
scalar.

Flattened objects and every reachable custom serializer use typed
`ProjectionDeclarationRow`s shared by serialization and scanning. For each
branch the declaration records its constructor and payload type, projected
surface, discriminator key and value computation, projected fields, flattened
nested dispatch, and typed derived keys; its `coordinate-origins` entries record
the exact source-to-coordinate mapping for those branches. Enum branches and
payloads are derived from the Rust type and must be exactly equal to the
declaration. The serializer uses the same declared key
constants and branch descriptors rather than a parallel spelling of keys such
as `type`, `relationParameter`, or `operatorDenotes`. Every manual serializer,
`serialize_with`, `flatten`, `transparent`, `untagged`, or other
shape-affecting serde form requires one such declaration; an unrecognized form
is an error. Constructor identity, payload identity, and serialized
representation remain separate facts.

The scanner first constructs the declared map from each `SemanticCoordinate` to
its one `CoordinateOrigin { semantic-coordinate, projection-id,
nonempty-source-origins }` and a reverse multimap from each `SourceOrigin` to
semantic coordinates. A second emission of the same coordinate, including from
a different source, is an error. A derived coordinate may depend on several
source origins, but all dependencies occur in its one coordinate origin. A
source origin may feed several coordinates only when its projection declaration
lists that exact set. Observed forward and reverse coverage must equal the
authored `coordinate-origins` exactly: unknown or stale origins, zero-covered
reachable origins, undeclared fan-out, missing output,
duplicate output, extra output, or wrong cardinality are errors. Only after
these checks is the coordinate key set compared with disposition owners; set
insertion never silently deduplicates coverage.

Raw origins consumed by a flattened projection are not emitted again as
generic coordinates unless the declaration assigns an additional semantic
distinction. Common fields may fan out across object branches because each
projected object field is a distinct coordinate and their coverage declaration
lists the full fan-out. Checked exclusions are outside the reachable-origin
multimap and form their own exact origin-keyed set. Successful semantic derived
facts may be coordinates. Layout choices belong to notation planning, while
planner/checker failures belong to the separate `AlgorithmFailure` owner
namespace described in section 14.2.

The exact gate is:

```text
scanner semantic coordinates
  == authored SemanticCoordinate disposition owners
  == minted SemanticCoordinate disposition owners
```

New model fields, variants, tuple payloads, flattened branches, discriminators,
or successful derived facts therefore fail the build until classified. No row
may be supplied by a wildcard, a surface-family default, Rust payload-shape
inference, or a mechanically repeated source coordinate. Each positive row
has at least one target leaf, every target leaf selects one closed target
contract, and every conditional row states a total decision over closed
projection facts with exact failure leaves. In particular,
the ledger must cover masses, quantities, relation/connective/tense/math values, argument
bundles, experience and locution events, every formula/connective family,
reciprocals, place/connective/tense questions, composition exclusions,
collective marking, scalar negation, intervals, arrays, math operator
denotation, approximate quantities, names and `goi`, every sign kind, paragraph
transitions, every utterance-force case, deictic proximity, recurrence,
actuality, personal mass membership, descriptor definiteness, pro-sumti,
plural quantifiers, centered/unordered intervals, all event facets, and every
displayed-content assertion effect.

The ledger also names source operations whose kernel route is easy to leave
implicit: `me` sumti-to-predicate conversion, `jai` event/place conversion,
`la'e` and `lu'e` reference/sign crossings, and relative-clause `ke'a` identity.
`ke'a` is `ProvenDesugaring` to the property parameter bound for the relative
clause. The other three use a verified typed reduction when the graph supplies
one and otherwise reach an explicit failure leaf; their semantic disposition
does not change merely because one guarded execution fails.
Resemblance to `me`, `Denotes`, or an event place is not itself an exact
reduction.

`tu'a` sumti raising is a named **tracked spec gap**. Its source semantics
deliberately withholds the particular abstraction: the raised operand stands
in for some unspecified abstraction about it, and no existing crossing can
express that without inventing content — an event abstraction requires the
very property `tu'a` withholds, `Abstract` requires known `Content`, and a
bare contextual computation loses the represented aboutness relation. Until
the semantic model's own commitment for `tu'a` is settled and a faithful
underspecified crossing is specified from it, every `tu'a` coordinate takes
the section-16 failure route; a renderer MUST NOT approximate it with a
plausible predicate, an invented abstraction kind, or a bare `Context`. The
same holds for `co'e` and `do'e`, whose meanings are deliberately vague in the
source language: vagueness that the model represents survives only through an
exact represented reduction, never through a guessed one.

Recording a coordinate as `ImplementationUnavailable` is acceptable for version
0; silent omission or an open wildcard is not.

Representative required decisions are:

| Owner | Disposition and total decision |
|---|---|
| `UtteranceForce::Assert` | `DirectLowering` to a typed `Assert Content -> (Act Assertion)` contract |
| `PredicationMode::Incidental` | `ProvenDesugaring` to anchored `Supplement` placement |
| `PredicationMode::Definitional` | if an exact registered definition is available, use it; otherwise projection fails at the `Content` position owned by the predication |
| `ArgumentValueKind::Filled` | row-preserving predicate fill |
| `ArgumentValueKind::Elided` | `NotationDefault` to the slot's registered `Close` computation |
| `ArgumentValueKind::Deleted` | exact `DropPlace` under registered deletion evidence |
| `UtteranceForce::Quote` | if the sign identity is available, desugar to `Mention : Sign<K> -> (Act Mentioning)`; otherwise projection fails at the `(Act Mentioning)` position owned by the utterance |
| `MathOperator::Add` | the closed `+ : Fn (Number Number) Number` kernel contract |
| unsupported `MathOperator::Power` | `ImplementationUnavailable`; the failure site is the `Number` position whose smallest owner is `MathExpression` |
| `ScopeDependence::Underspecified` | `(Context dependency...)` using the exact ordered binder identities |
| `AlgorithmFailure::RootNotPerformable` | invalid graph; `WholeGraph { SemanticGraph }` failure site with reason `smusni.projection.graph.root-not-performable` |
| `AlgorithmFailure::ContextUnboundVariable` | invalid graph; `WholeGraph { SemanticGraph }` failure site with reason `smusni.projection.graph.unbound-variable` |

These are physical per-coordinate rows, not family rules. A field coordinate
which selects among variants normally targets its typed assembly/dispatch
algorithm, while each variant coordinate selects the exact operation or
conditional route it denotes.

In particular, displayed-content `TargetFocus` is `ProvenDesugaring` from the
first-class target identity/type; its family selects the registered relation;
and its assertion effect is represented by the assembled act structure.
`CommandTarget` marker provenance is suppressed only after the directive
addressee has been recovered from the graph's argument fill. Scope-dependence
sets, formula/sequence event-binding scope, relation expansions, and rafsi
bindings likewise require individual ledger entries: semantic dependencies and
scope lower normally, while spelling/source expansion data may be suppressed
only with a declared provenance reason.

## 15. Canonical rendering

Canonical output is deterministic from the typed projection graph.

1. Bindings are planned before printing. Pure shared acyclic values use the
   smallest lexical body containing every use. Dynamic references use the exact
   placement rules in section 6.3. Unshared values inline only when their
   normal-form rule permits it.
2. Strongly connected components of guarded inert functions print as one
   `LetRec`; any other semantic cycle makes projection fail with a registered
   reason. Eligible SCCs and
   members use stable graph order with source span as the first key and stable
   object id as the second; absent spans sort after present spans by stable
   object id. Semantic operands are never sorted merely because an operator is
   mathematically commutative.
3. Variables are alpha-renamed by first binder occurrence. A safe graph-owned
   semantic label is preferred when present. Otherwise deterministic type/role
   stems are used: `$x` for entities/references, `$e` for events, `$p` for
   predicate/function values or places, `$q` for queries or generalized
   quantifiers, `$u` for utterances, `$s` for signs, and `$v` otherwise; decimal
   suffixes avoid collision. Descriptive names in `samples.md` are pedagogical
   alpha-renamings, not byte-for-byte renderer expectations.
4. Plain predicate operands print until a skip is necessary. Literal labelled
   fills then follow the cursor rules. Computed fills print `At`. Event fills
   print after numbered fills unless source/effect order requires an earlier
   named event.
   A registered exact-cardinality equality prints its `Card` expression on the
   left and finite numeric literal on the right; this is a rule of that
   reduction, not permission to reorder arbitrary equality operands.
5. Associative operators flatten in graph order. They never reorder dynamic
   operands. Redundant one-item wrappers contract only where specified.
6. Strings use JSON escaping, symbols and text are NFC, and exact numbers use
   the rules in section 2.
7. The pretty-printer uses two-space indentation and deterministic line
   breaking. Line width and the choice to break an otherwise short list are not
   version-0 semantic conformance requirements while the notation is
   experimental; consumers MUST ignore whitespace. Every implementation MUST
   nevertheless render the same graph identically under one fixed formatter
   configuration.
8. Output ends with one newline and contains no trailing whitespace.

Whitespace-insensitive parsers may accept any layout satisfying section 2.1.

## 16. Result contract and projection failure

### 16.1 The two results

Projection is fallible. A render either succeeds or fails, and the two results
have disjoint shapes:

```text
Success = one complete (Smusni 0 ...) document
        + ordered nonfatal diagnostics
        + projection statistics
Failure = no document
        + a nonempty ordered list of registered projection diagnostics
        + projection statistics
```

The normative points are:

- a successful result contains exactly one complete `(Smusni 0 ...)` datum. It
  is well-typed under section 2.2 and the registered signatures, and it uses
  only the compact normal forms this document specifies;
- a projection failure contains no datum, no partial datum, no `Words` section,
  and no serialized graph payload. There is nothing for a consumer to parse and
  nothing for it to mistake for a document;
- each failed projection edge produces exactly one registered record. Ordering
  is deterministic for a given graph, and a failing wrapper does not duplicate
  the records of its children;
- every record carries a stable reason id from the closed
  `smusni.projection.` namespace, a stable message fixed by its typed cause,
  severity `error`, a primary source span, and typed owner and use-site
  evidence when the graph supplies them;
- diagnostics and statistics are host-neutral structured data. Terminal text is
  a presentation of them, never their definition, and a host MUST NOT recover
  structure by parsing rendered text back;
- nonfatal semantic warnings stay separate from projection errors. On success
  they accompany the document; on failure they may accompany the failure
  envelope but never substitute for its nonempty error list. Whether a
  graph-attached diagnostic of severity `error` independently prevents success
  is a decision the implementation states explicitly; projection becoming
  fallible does not silently promote every warning; and
- warnings and errors MUST NOT appear inside the datum. Section 2.4 already
  forbids it, and this section is their channel.

Statistics are API data unless a host separately requests them. They retain the
failed-edge count, the per-reason counts, and the number of distinct failing
owners, because those are what corpus tooling measures.

### 16.2 Failure ownership and attribution

A projection failure names the smallest semantic owner it can prove. The
failing edge is identified by its typed identity, not by a rendered string, and
the failure record carries its resolved source location at the moment the
failure is created; late formatting never has to rediscover provenance.

Source attribution uses the first rule that applies:

1. the primary span of the failed semantic edge or its owner;
2. otherwise the span of the use site;
3. otherwise the span of the nearest source-bearing semantic ancestor that the
   planner or elaborator held when the failure was created;
4. only for a genuinely source-free internal failure, the whole input span,
   with the owner and use-site identities carried as notes rather than as a
   substitute for a label.

Each failure is classified, and the classification is part of the record:

- **invalid semantic graph.** The graph is malformed, ambiguous where this
  document requires an exact choice, or internally inconsistent — an unbound
  variable, an ill-scoped witness request, an impossible effect handler, a
  requested but illegal de-re owner. Such a graph is rejected. Nothing follows
  about whether smusni can express the corresponding distinction;
- **implementation route unavailable.** The coordinate has a normative semantic
  disposition under section 14.4, and this implementation does not yet carry
  it. This is renderer backlog;
- **tracked spec gap.** Version 0 specifies no route for the coordinate at all.
  This is language-design backlog, and section 14.4 forbids counting it toward
  expressive completeness;
- **implementation invariant failure.** The implementation itself broke — an
  internal invariant, registry/executable drift, or a failure of an algorithm
  the registry declares total. This is a defect to fix, never a semantic
  result; it is reported honestly rather than laundered into one of the other
  classes.

These are the same four classes section 14.4 requires reports to separate.

Failures aggregate rather than escalate: one graph may fail at several
independent edges, and all of them are reported. Escalation to a larger owner
happens only when no unique smaller owner contains the failed dependency
closure, exactly as section 14.2 requires of a registered failure site. The
no-document invariant is absolute — there is no owner large enough that failing
at it produces a document.

### 16.3 Host profiles

Hosts present the same structured result differently. Each profile is stated at
outline level here; the shared structured diagnostic schema in section 16.1 is
the contract, and a host adds presentation, never meaning.

- **command line.** Success writes the document to stdout and any nonfatal
  diagnostics to stderr. Failure writes nothing to stdout, writes the labelled
  diagnostics to stderr through the same standard source-aware renderer the
  rest of the toolchain uses, and exits nonzero. The rendered source is the
  original Lojban; owner and use-site identities are secondary notes on a
  label, not replacements for one. A display limit MAY truncate the printed
  records only if it also prints how many were omitted;
- **HTTP.** Failure is a server-side capability failure, not a malformed
  request: the request validly asked for smusni and the server could not
  fulfill it.
  Return a machine-readable problem document carrying a stable top-level code,
  the requested format, the diagnostic list, and the statistics. Malformed
  request or input keeps its ordinary client-error status;
- **MCP.** Failure returns the tool's error result with a readable summary
  first, and carries the same serializable envelope the HTTP profile uses.
  Structured content is declared by a matching output schema or serialized as
  one JSON text item; it is never a terminal transcript.

A transport MAY impose a documented safety limit on how many records it
returns, and then MUST report the total, the returned count, and that
truncation occurred. The internal failure always holds every record in
deterministic order.

An explicit smusni request is never silently retried in another format. A
caller that asked for smusni and received a failure asked a question that was
answered honestly; answering it with XML instead would misreport what was
produced.

## 17. Validation requirements

Version-0 conformance requires structural and semantic validation, not golden
output expectations. The first two groups are the laws of the two results in
section 16; the rest are the semantic checks that make a successful result
mean what it says.

**Every successful result:**

- is one parseable, well-typed `(Smusni 0 ...)` datum, and uses only the
  compact public grammar of section 2.2;
- is byte-identical on repeated rendering under one formatter configuration;
- binds every variable exactly once and uses it within its legal scope;
- carries no diagnostic node in the document, and collects its nonfatal
  diagnostics once in stable order.

**Every failed result:**

- contains no document, no partial document, no `Words` section, and no
  serialized graph payload;
- contains at least one projection error, and every one of them resolves to a
  registered reason id in the `smusni.projection.` namespace with its stable
  message, severity `error`, and a source span selected by the section-16.2
  attribution rules;
- reports its errors in an order that is deterministic for a given graph, with
  one record per failed edge and no wrapper duplicating a child's record;
- is classified per record into the four section-16.2 classes, so that a
  corpus report can separate renderer backlog from language backlog and both
  from invalid input and implementation defects;
- reaches every host transport unchanged in meaning: a nonzero exit and empty
  stdout on the command line, a server-error problem document over HTTP, and a
  tool error result over MCP, each carrying the same structured records rather
  than a reparsed transcript, and each reporting the total when it truncates
  what it displays;
- is never silently replaced by a result in another format.

**Registry and semantic checks:**

- the checked-in registry validates every primary key, foreign key, closed
  enum, and template, and two clean generator runs produce byte-identical
  artifacts which `generate --check` compares against the checked-in bytes;
- every kernel summary agrees with sections 3, 6, and 14.1, every prelude row's
  declared type parameters, signature, and canonical definition match
  the mechanically extracted and registry-metaform-normalized definition in
  this document, every recursive `PureProperty` refinement is retained through
  partial application and binding and enforced at the eventual application,
  every prelude/tag/relation-former
  summary is rederived from its expansion or link contract, and every
  irreducible generated relation has one complete
  type/context/effect/stability row;
- every shared identity is represented once by `Let`, `Bind`, `LetRec`, or a
  token binder;
- every predicate fill resolves against the current row and every lexical
  policy lookup resolves through original-slot provenance;
- every implicit `Close` is reconstructible from syntax, expected type, and
  registered row defaults, including one distinct fixed `Context` computation
  per omitted place and reuse of that same lexical closure-site identity across
  repeated applications within one performance;
- every computed `At` has a candidate domain that is either exactly
  reconstructible at its host or explicit, excludes all whole-application
  reservations, and is disjoint from other computed fills;
- every prelude name expands to the specified `Let` definition with the same
  type, evaluation order, effects, and sharing;
- every `RefComp` node has exactly one legal lexical/dynamic host and every side
  effect is handled once; runtime evaluation follows that host, so a host under
  a quantifier or a repeatedly performed act may invoke its computation once
  per represented instantiation or performance;
- the accessibility table is exercised for every connective, quantifier,
  reification boundary, and discourse operation;
- every generalized-quantifier witness is retrieved only after the same `GQ`
  and nuclear-scope application succeeds and proves a nonempty export, including
  rejection after ambiguous reusable-act reperformance;
- importing universal behavior and its dependency-legal presupposition handler
  are preserved under negation and in a supported simultaneous termset;
- equal-scope termsets are never silently converted to ordered nesting;
- every tense, space, aspect, recurrence, and actuality node either expands by
  its registered exact lexical/compositional reduction or has an explicit
  coverage row saying which implementation route is unavailable;
- every generated callable spelling is globally unambiguous in value position
  and resolves without consulting a nonprinting family or overload tag;
- all query domains and answer selections typecheck;
- overlapping multi-`At` candidate domains fail locally rather than accepting a
  noninjective answer assignment;
- utterance/sign facts refer to their bound token and performance remains
  distinct from reported facts;
- the recursively resolved semantic-model source closure is exact; every
  reachable or designated type origin, constructor, scalar-code variant,
  named field, tuple payload, custom discriminator, flattened field, and
  successful semantic derived fact is emitted exactly once and classified by
  the exhaustive disposition ledger, while checked exclusions are exact and
  reasoned; aliases alone have empty coordinate-origin mappings and contribute
  no semantic coordinates, every other semantic type has a nonempty mapping,
  every semantic coordinate is globally unique and has its exact nonempty
  source-origin set, and every source origin's exact coordinate fan-out equals
  the authored `coordinate-origins` mapping;
- every semantic disposition decision is total, every projection fact has one
  pure generated evaluator, every target contract has one typed implementation,
  every projection algorithm has one generated executor, and authored fact,
  algorithm, raw-model-type, and algorithm-failure rows equal their generated
  closed runtime variants exactly;
- every algorithm failure site belongs to exactly one declared algorithm,
  names one compatible owner input, is in that algorithm's exact failure
  set, transfers only through its own failure-only coverage row, and is
  exercised by a focused negative fixture;
- every fact and target use supplies exactly the declared typed inputs and type
  arguments, every target path entails its own and its underlying algorithm's
  required facts, and fact, algorithm, target, reason, projection, exclusion,
  and disposition rows have neither unresolved references nor orphans;
- the complete instantiated fact/target/algorithm/lowered-value dependency
  graph is acyclic, and every transitive local algorithm failure resolves to
  one type-compatible enclosing owner;
- every `TypedPosition` reason resolves to one registered expected type and
  smallest owner, while every `WholeGraph` reason resolves to a site with no
  expected smusni type and root exactly `SemanticGraph`;
- every semantically valid reachable coordinate has one of the five semantic
  dispositions of section 14.4, and coordinates recorded as tracked spec gaps
  are enumerated separately and never counted as expressive completeness;
- failure counts, reason ids, and the four failure classes of section 16.2 are
  reported on representative CLL and corpus suites without imposing an
  experimental threshold.

Focused registry mutations must independently reject:

- a new, removed, unresolved, or conditionally hidden production model module,
  or a production-configuration-dependent semantic surface;
- a new or removed reachable type, constructor, scalar variant, named field, or
  tuple payload, regardless of visibility;
- a field or payload type change with the same name and count, tuple reorder or
  type substitution, alias/container target or reachability change, or explicit
  `Sum`/`ScalarCodeList` reclassification;
- an unprojected or stale flattened branch, changed projected field,
  non-discriminator key, discriminator, derived key, or value mapping, or a
  serializer which stops consuming its shared declaration;
- an unregistered shape-affecting serde/custom-serializer change, duplicate
  authored coordinate across projection rows, duplicate coordinate emission,
  swapped or otherwise wrong coordinate-to-origin map, wrong reverse origin
  fan-out, zero-covered reachable origin, unknown/stale origin, missing/extra
  projection output, an empty non-alias coordinate mapping, or a nonempty alias
  coordinate mapping;
- a missing, duplicate, unjustified, or stale exclusion, or confusion between
  semantic derived facts, notation facts, and algorithm failure sites;
- an unknown, duplicate, or orphan target, fact, reason, algorithm, projection,
  or failure-site identifier; a fact-owner/input mismatch; a target
  input/result mismatch; an unmet target/algorithm requirement on any decision
  path; an algorithm returning an undeclared site; a site attached to the wrong
  algorithm or owner input; or a wrong structured reason-owner join;
- an invalid `TypedPosition` or `WholeGraph` failure site, or a declared owner
  which disagrees with that site;
- a missing nested cardinal-GQ purity refinement, an unjustified refinement, a
  duplicate generated callable spelling in any family, or a collision with an
  existing value-position atom; and
- a cycle through `RecoveredAt`, registry expansion, fact/target/algorithm
  bindings, or lowered-value dependencies; or generated-runtime drift.

Positive mutation fixtures also enumerate every current object branch,
dispatch branch, tuple payload, custom projection declaration, exclusion,
successful derived fact, and algorithm failure site so a lower-bound-only scan
cannot pass.

The corpus suite must include complex relative clauses, multiple connected
relatives, inner and outer-`ku` `poi` with and without an explicit outer
quantifier, a greatest-subreference case, inner `noi`, `noi` under negation,
alternatives, and questions, a `noi` side depending on a quantified variable,
supplements inside mentioned and reified content, `goi`, all connective loci,
compound and negated tags, arbitrary
`fi'o`, abstractions with extra places, shared event facets, de-re/de-dicto and
opaque references, direct and embedded questions, multi-variable questions,
termsets, respectively, recursion, quotations and every sign kind, indicators,
multiple utterances, and math/quantity structures. It must also include inputs
which fail, covering the first three of section 16.2's classifications, so the
failure laws above are exercised on real graphs rather than only on fixtures.
(An implementation invariant failure is by construction not reachable from a
valid corpus input; it is exercised by focused negative fixtures instead.)

The internal debug codec described in `internal-raw.md` has its own tests. They
are explicitly not conformance tests: nothing they accept is a smusni document,
and a consumer never sees their output.

## 18. Summary of the model

The normal form is “predicate terms and lambdas all the way down” at the
application-graph layer, with a small number of irreducible semantic controls:

- predicate-place filling;
- lambda/function application;
- contextual closure;
- dynamic binding and effect handling;
- exact logical and quantificational control;
- act construction and performance;
- graph-owned token identity;
- explicit semantic level crossings.

Everything else should be decomposed into those operations and ordinary
predications when the decomposition preserves the semantic graph. A named
record constructor is warranted only when eliminating it would lose a real
type, scope, effect, identity, order, or interpretation distinction.

Every valid semantic distinction has a route through those controls, the
lexical predicates, and the reviewed transparent reductions. There is no
opaque node to fall back on: an implementation that cannot prove a faithful
projection returns a product projection error and emits no document, and a
distinction with no route at all is a tracked spec gap to be closed rather than
a shape the format quietly tolerates.
