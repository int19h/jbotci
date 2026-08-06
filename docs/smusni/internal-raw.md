# Internal raw codec (non-normative)

**This document is not part of the smusni format.** Nothing described here is a
smusni document, and no consumer of the format should ever be handed one. The
public specification is [`spec.md`](spec.md); its section 16 says that a
projection either produces one complete `(Smusni 0 ...)` document or produces
no document at all.

What is described here is a jbotci-internal debug and test codec. It exists for
three reasons, all of them internal:

- a losslessness oracle. Serializing a semantic graph and reading it back
  proves that a payload, a sharing edge, or a cycle survives, which is what
  makes the compact projections trustworthy;
- a debug capture. When a projection fails, dumping the smallest failing owner
  is a fast way to see what the planner actually held; and
- corpus tooling. Reason distributions and coverage measurements are computed
  from typed internal values, not from parsed output.

The codec is unstable, unversioned, and may change in any commit. It is not a
compatibility surface, and its bytes are not smusni bytes. If a stable
interchange format for whole graphs is ever wanted, it should be specified and
versioned as a different format; the existing canonical JSON and XML
representations are better starting points than promoting these debug
S-expressions.

If a developer-facing flag ever exposes this codec, it carries an unmistakable
name such as `--debug-smusni-raw`, stays out of product and tool schemas by
default, and says in its own help text that its output is not smusni.

## Historical reason-id migration

Before the change recorded in issue #753, this project emitted the reason
namespace `smusni.fallback.` and treated an unproved projection as a conforming
raw document. Both are gone: the namespace is now `smusni.projection.` and an
unproved projection is a product error.

The rename is a pure prefix substitution — `smusni.fallback.` became
`smusni.projection.` and every suffix is unchanged — so a historical corpus
report can be compared with a current one by rewriting the prefix. The complete
map is given below anyway, because a rule is easy to misapply and these ids
appear in archived measurements.

Pre-#753 experimental `TypedGraph` and `Fallback` output is removed and
unsupported. There is no migration path for a document that contained one:
those bytes were never a conformant v0 normal form, since version 0 had not
been minted, and the input should be re-rendered.

| Historical id | Current id |
|---|---|
| `smusni.fallback.abstraction-crossing-unlicensed` | `smusni.projection.abstraction-crossing-unlicensed` |
| `smusni.fallback.actuality.demonstrated` | `smusni.projection.actuality.demonstrated` |
| `smusni.fallback.actuality.potential` | `smusni.projection.actuality.potential` |
| `smusni.fallback.binder-does-not-dominate-use` | `smusni.projection.binder-does-not-dominate-use` |
| `smusni.fallback.computed-fill-domain-noninjective` | `smusni.projection.computed-fill-domain-noninjective` |
| `smusni.fallback.conflicting-binder-owners` | `smusni.projection.conflicting-binder-owners` |
| `smusni.fallback.de-re-owner-dependency-illegal` | `smusni.projection.de-re-owner-dependency-illegal` |
| `smusni.fallback.de-re-owner-missing` | `smusni.projection.de-re-owner-missing` |
| `smusni.fallback.de-re-owner-opaque` | `smusni.projection.de-re-owner-opaque` |
| `smusni.fallback.de-re-owner-unrelated-or-nondominating` | `smusni.projection.de-re-owner-unrelated-or-nondominating` |
| `smusni.fallback.de-re-owner-wrong-kind` | `smusni.projection.de-re-owner-wrong-kind` |
| `smusni.fallback.declaration-planning-nonconvergence` | `smusni.projection.declaration-planning-nonconvergence` |
| `smusni.fallback.definition-site-does-not-dominate-use` | `smusni.projection.definition-site-does-not-dominate-use` |
| `smusni.fallback.dependent-supplement-unrepresentable` | `smusni.projection.dependent-supplement-unrepresentable` |
| `smusni.fallback.descriptor.unlowered-sumti` | `smusni.projection.descriptor.unlowered-sumti` |
| `smusni.fallback.dynamic-host-cycle` | `smusni.projection.dynamic-host-cycle` |
| `smusni.fallback.dynamic-host-not-unique` | `smusni.projection.dynamic-host-not-unique` |
| `smusni.fallback.effect-handler-missing-or-illegal` | `smusni.projection.effect-handler-missing-or-illegal` |
| `smusni.fallback.event-facet-reduction-unregistered` | `smusni.projection.event-facet-reduction-unregistered` |
| `smusni.fallback.event-owner-missing-or-nonunique` | `smusni.projection.event-owner-missing-or-nonunique` |
| `smusni.fallback.force-handler-missing-or-illegal` | `smusni.projection.force-handler-missing-or-illegal` |
| `smusni.fallback.force-reduction-unrepresentable` | `smusni.projection.force-reduction-unrepresentable` |
| `smusni.fallback.generated-eventuality-unbound` | `smusni.projection.generated-eventuality-unbound` |
| `smusni.fallback.higher-order-crossing-unlicensed` | `smusni.projection.higher-order-crossing-unlicensed` |
| `smusni.fallback.lexical-policy.entity` | `smusni.projection.lexical-policy.entity` |
| `smusni.fallback.lexical-policy.eventuality` | `smusni.projection.lexical-policy.eventuality` |
| `smusni.fallback.lexical-relation-row-missing` | `smusni.projection.lexical-relation-row-missing` |
| `smusni.fallback.lexical-signature-missing-or-stale` | `smusni.projection.lexical-signature-missing-or-stale` |
| `smusni.fallback.math-reduction-unregistered` | `smusni.projection.math-reduction-unregistered` |
| `smusni.fallback.math.array` | `smusni.projection.math.array` |
| `smusni.fallback.math.base` | `smusni.projection.math.base` |
| `smusni.fallback.math.centered-interval` | `smusni.projection.math.centered-interval` |
| `smusni.fallback.math.operator-denotation` | `smusni.projection.math.operator-denotation` |
| `smusni.fallback.math.power` | `smusni.projection.math.power` |
| `smusni.fallback.math.questioned-operator` | `smusni.projection.math.questioned-operator` |
| `smusni.fallback.math.subscript` | `smusni.projection.math.subscript` |
| `smusni.fallback.math.unordered-interval` | `smusni.projection.math.unordered-interval` |
| `smusni.fallback.modal-tag-reduction-unregistered` | `smusni.projection.modal-tag-reduction-unregistered` |
| `smusni.fallback.place-deletion-evidence-missing` | `smusni.projection.place-deletion-evidence-missing` |
| `smusni.fallback.predicate-closure-unlicensed` | `smusni.projection.predicate-closure-unlicensed` |
| `smusni.fallback.predicate-fill-type-or-arity-mismatch` | `smusni.projection.predicate-fill-type-or-arity-mismatch` |
| `smusni.fallback.prelude-reduction-unavailable` | `smusni.projection.prelude-reduction-unavailable` |
| `smusni.fallback.quantifier-effect-export-illegal` | `smusni.projection.quantifier-effect-export-illegal` |
| `smusni.fallback.quantity-reduction-unregistered` | `smusni.projection.quantity-reduction-unregistered` |
| `smusni.fallback.quantity.approximate` | `smusni.projection.quantity.approximate` |
| `smusni.fallback.quantity.enough` | `smusni.projection.quantity.enough` |
| `smusni.fallback.quantity.too-few` | `smusni.projection.quantity.too-few` |
| `smusni.fallback.quantity.too-many` | `smusni.projection.quantity.too-many` |
| `smusni.fallback.question-domain-or-answer-mismatch` | `smusni.projection.question-domain-or-answer-mismatch` |
| `smusni.fallback.reference-description-unrepresentable` | `smusni.projection.reference-description-unrepresentable` |
| `smusni.fallback.relation-former-reduction-unavailable` | `smusni.projection.relation-former-reduction-unavailable` |
| `smusni.fallback.relation-reduction-unregistered-or-inexact` | `smusni.projection.relation-reduction-unregistered-or-inexact` |
| `smusni.fallback.relation.constructed` | `smusni.projection.relation.constructed` |
| `smusni.fallback.scope-dependency-without-binder` | `smusni.projection.scope-dependency-without-binder` |
| `smusni.fallback.sequence-reduction-unregistered` | `smusni.projection.sequence-reduction-unregistered` |
| `smusni.fallback.sign-identity-missing` | `smusni.projection.sign-identity-missing` |
| `smusni.fallback.simultaneous-termset-unlicensed` | `smusni.projection.simultaneous-termset-unlicensed` |
| `smusni.fallback.structured-quotation-transcript-entry-missing` | `smusni.projection.structured-quotation-transcript-entry-missing` |
| `smusni.fallback.unguarded-or-unrepresentable-scc` | `smusni.projection.unguarded-or-unrepresentable-scc` |
| `smusni.fallback.unknown-registry-coordinate` | `smusni.projection.unknown-registry-coordinate` |


## Codec structure

The rest of this document records the structural rules the codec follows. They
are retained because the losslessness oracle depends on them, not because any
consumer may rely on them.

### Local owner capture

When the smallest failing owner has a known expected type, a capture prints as
`(Fallback expected-type "reason-id" raw-value)`. The example below shows a
complete payload-preserving capture for the registered
`smusni.projection.math.power` reason.

The first operand is ordinary unquoted type syntax from section 2.2 of the
specification and records the smusni type position that failed. The reason id
is a stable ASCII string registered by the `ProjectionFailureReasonRow` table
and begins with the closed namespace prefix `smusni.projection.`; it is the
same id the structured failure record carries. A failure which occurs while
determining that very type has no sound local position and is captured at the
nearest owner with a known expected type, or at the whole graph. The printed
expected type and first raw value's model type equal the registered failure
site's expected type and minimum owner respectively.

The raw grammar is closed:

```text
(Object %id "type-name" (Field "field-name" raw)*)
(Ref %id)
(RawRecord "type-name" (Field "field-name" raw)*)
(RawVariant "enum-type" "constructor" (Field "payload-name" raw)*)
(RawList raw*)
(RawMap (Entry raw-key raw-value)*)
(RawTypedAtom "scalar-code-enum-type" "case")
(RawScalar "model-scalar-type" "lexical-value")
(RawAtom "exact-atom")
(RawString "text")
(RawNull)
```

Object identities use the `%1`, `%2`, ... namespace, which exists only inside a
capture and has no counterpart in the public grammar. Within each capture root,
every graph object identity is assigned one `%id` in first-encounter depth-first order. Its first encounter
MUST be the corresponding `Object`; later sharing and cycle back-edges use
`Ref`. A separate capture root restarts at `%1`, and a `Ref` never crosses raw
roots. `Object` is reserved for identity-bearing `SemanticGraph` and
`SemanticObject` values and is the only constructor which introduces an object
ID. `RawRecord` preserves an inline product or newtype with no graph identity.
`RawVariant` preserves an algebraic-sum constructor: unit constructors have no
payload fields, named payloads use stable semantic field names, and tuple
payloads use their projection-declared stable names rather than Rust source
indices. `RawTypedAtom` is reserved for a declared scalar-code enum and cannot
carry a payload. `RawScalar` preserves a model scalar/newtype family plus its
exact lexical representation. `RawMap` preserves typed keys and values as
ordered entries instead of collapsing keys to strings; `RawList` preserves an
ordered collection. Fields, variant payloads, and entries occur in the model or
projection declaration's canonical order. `RawAtom` preserves one genuinely
untyped atom spelling. `RawNull` occurs only for an absent optional value whose
absence must be retained. Every owned nested value is representable by exactly
one raw form. Model type, constructor, member, and atom names remain strings so
they cannot escape into the normal PascalCase namespace.

Raw traversal begins only after applying the ledger's nonsemantic routes.
Every `DiagnosticCollection` coordinate is enqueued exactly once on the ordered
diagnostic channel and replaced in the raw view by its declared canonical empty
value; in particular, `SemanticObjectCommon.diagnostics` becomes `(RawList)`,
so a capture never carries a diagnostic code or message. An
`OmitNonsemantic` provenance coordinate is likewise replaced by its declared
canonical absent or empty raw value, and a `RecoveredAt` coordinate is emptied
only after its value is retained at the declared target. This preprocessing is
the explicit exception to structural raw preservation. After it, every
remaining semantic value and every canonical empty placeholder is encoded
exactly once, preserving the product shape without duplicating diagnostics or
printing source-only provenance.

For example, these fragments preserve a payload enum, a named payload with
shared graph identities, and a typed-key map:

```lisp
(RawVariant "MathOperator" "Named"
  (Field "name" (RawString "custom-op")))
(RawVariant "ScopeDependence" "Underspecified"
  (Field "mayDependOn"
    (RawList (Ref %2) (Ref %5))))
(RawMap
  (Entry (RawScalar "PlaceIndex" "x1") (Ref %7)))
```

### Whole-graph capture

If no well-typed local position exists, the whole graph is captured:

```lisp
(TypedGraph "SemanticGraph" "smusni.projection.graph.unbound-variable" raw-root)
```

`TypedGraph raw-root-type reason-id raw-root` appears only as a capture root.
Its reason id resolves to a `WholeGraph` failure site with the same root type,
and the first raw value is `Object %1` with that same type name. An unbound
variable, an ill-scoped witness, an explicitly requested but invalid de-re
owner, or an impossible effect host reaches this capture when no smaller typed
position exists; there are no underspecified `Unbound` or `IllScoped` semantic
values. An unknown model or disposition coordinate is registry or executable
drift and fails before rendering rather than becoming a capture at all.

The quoted first operand is a model root-type name rather than smusni type
syntax, because no smusni type was established for it. The separate quoted
reason is still recorded so that a capture and the structured failure record it
accompanies name the same registered failure.

## Captured examples

These two captures were previously published as section 20 of `samples.md`.
They are debug output, not samples of the format.

A local capture preserves its type, reason, fields, identity, and sharing:

```lisp
(Fallback Number "smusni.projection.math.power"
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
```

A whole-graph capture preserves the graph structurally:

```lisp
(TypedGraph "SemanticGraph" "smusni.projection.graph.root-not-performable"
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
          (Ref %2))))))
```

Neither capture is a document, and neither reaches a consumer. The
corresponding structured projection error is what a host reports.

## Tests

The codec's tests cover raw round-tripping, identity and sharing preservation,
`RawVariant`, `RawRecord`, and `RawMap` payload preservation, smallest-owner
selection, and capture-reason counts. None of them is a conformance test:
success for this codec means the internal oracle held, not that anything
emitted is a valid smusni document.
