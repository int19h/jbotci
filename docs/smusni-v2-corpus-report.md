# Experimental smusni v2 corpus report

This report records structural observations for the issue #741 implementation.
The examples below are review aids, not golden expectations. The test suite
parses renderer output and checks typed structure, binding, references, field
coverage, diagnostics, and determinism without comparing whole output strings.

## Method

The reproducible reporter is
`crates/jbotci-semantics/examples/smusni_corpus_report.rs`. It runs the normal
morphology, syntax, semantic-builder, and smusni-renderer pipeline under panic
boundaries and reports renderer-provided mode and fallback counters. The
measurements below were made from commit `dfc78d3785` in release mode with:

```text
CARGO_TARGET_DIR=/build/jbotci/target/issue-741 cargo build -r -p jbotci-semantics --example smusni_corpus_report
/build/jbotci/target/issue-741/release/examples/smusni_corpus_report phaseb
/build/jbotci/target/issue-741/release/examples/smusni_corpus_report cll
/build/jbotci/target/issue-741/release/examples/smusni_corpus_report focused
/build/jbotci/target/issue-741/release/examples/smusni_corpus_report alice-lines
/usr/bin/time -v /build/jbotci/target/issue-741/release/examples/smusni_corpus_report alice-whole
```

The inputs are:

- all 48 retained Phase-B semantic graph source documents;
- all 1,247 nonempty `lojban` entries under `tests/fixtures/cll`;
- 16 focused inputs covering `zi'o`, paragraphs, questions, quotations and
  hostile strings, `poi`/`noi`, complex connectives, abstractions, termsets,
  both respectively surfaces, modals, tense, math, and displayed content;
- all 2,436 nonempty physical lines of the zantufa Alice corpus, independently;
- the complete Alice corpus as one multi-utterance document.

## Results

Percentages use successfully built graphs as their denominator. “Object” means
a compact document containing one or more complete local typed object
fallbacks. Build failures are reported separately and were never hidden as
renderer outcomes.

| Corpus | Inputs | Built and rendered | Build failures | Build panics | Render panics | Compact | Object | TypedGraph | Warnings |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Phase B | 48 | 48 | 0 | 0 | 0 | 24 (50.0%) | 10 (20.8%) | 14 (29.2%) | 2 |
| Focused | 16 | 16 | 0 | 0 | 0 | 13 (81.2%) | 3 (18.8%) | 0 (0.0%) | 3 |
| CLL fixtures | 1,247 | 1,245 | 2 morphology | 0 | 0 | 316 (25.4%) | 369 (29.6%) | 560 (45.0%) | 100 |
| Alice lines | 2,436 | 1,084 | 5 morphology, 1,339 syntax, 8 semantics | 0 | 0 | 124 (11.4%) | 153 (14.1%) | 807 (74.4%) | 680 |
| Alice whole | 1 | 1 | 0 | 0 | 0 | 0 | 0 | 1 (100%) | 780 |

Every successfully built graph rendered. No renderer panic occurred in any
slice. Alice physical-line failures are upstream parsing or building failures;
the complete Alice document builds successfully, so it is the representative
multi-utterance renderer measurement.

The most frequent fallback reasons were:

| Corpus | Most frequent reasons (count) |
| --- | --- |
| Phase B | `referent-fields` 16; `definition-site-does-not-dominate-use` 15; `unrepresentable-local-binder` 8; `unrecognized-object-family` 7 |
| Focused | `referent-fields` 2; `eventuality-facets` 1; `predication-side-fields` 1 |
| CLL fixtures | `definition-site-does-not-dominate-use` 622; `referent-fields` 418; `unrepresentable-local-binder` 325; `eventuality-facets` 239; `predication-side-fields` 150; `unrecognized-object-family` 134 |
| Alice lines | `unrepresentable-local-binder` 1,162; `definition-site-does-not-dominate-use` 979; `referent-fields` 677; `unrecognized-object-family` 475; `eventuality-facets` 313; `non-atomic-relation` 312 |
| Alice whole | `scope-dependency-without-enclosing-binder` 266; `binder-does-not-enclose-use` 101; `multiple-binder-owners` 9 |

The high TypedGraph rate is a safety result, not a compactness target. In
particular, known issue #742 affects lexical dominance and scope dependency in
the current builder, while #743 tracks prenex/termset builder structure. The
renderer does not repair either graph shape. The complete Alice graph has
49,172 objects and deterministically selects TypedGraph from the three proven
scope-failure classes above.

The direct prebuilt-binary Alice-whole run took 33.07 seconds wall time and
30.87 seconds user time. Immediately after semantic graph construction it used
7,523,088 KiB RSS with a 7,861,212 KiB high-water mark. After rendering it used
7,576,292 KiB RSS and the high-water mark was unchanged. Thus the measured
~7.5 GiB allocation already exists at the builder/render boundary; smusni adds
about 53 MiB resident and does not raise the process peak. The planner stops
after binder/universe failures prove TypedGraph necessary, so it does not run
SCC or definition-placement work that TypedGraph cannot consume.

## Selected observations

These outputs were selected to make specific semantic decisions easy to
review. They are observations from the release binary, not test fixtures.

`mi pu klama lo zarci` retains the generated matrix event and its time facet:

```lisp
(Smusni
  0
  (Assert
    (∃
      (($eventuality_6 Eventuality))
      (∧
        (klama
          Speaker
          (Lo zarci)
          (At Eventuality $eventuality_6))
        (Before $eventuality_6 Now)))))
```

`mi djica lo nu mi pu cilre` keeps the embedded event inside the description
and anchors its `Before` facet to the matrix event:

```lisp
(Lo
  (($eventuality_7 Eventuality))
  (Nu $eventuality_7 (cilre Speaker))
  (Before $eventuality_7 $eventuality_6))
```

`le gerku poi blabi cu melbi` uses one lexical description binder shared by
the base property and relative clause:

```lisp
(Le
  (($entity_7 Entity))
  (gerku $entity_7)
  (Relative Restrictive Veridical (blabi $entity_7)))
```

`mi klama sepi'o lo karce` renders the actual converted adjunct predicate/place
map. Both filled places are labelled, including the graph-filled event place:

```lisp
(Modal
  (pilno
    (At 2 (Lo karce))
    (At 3 $eventuality_6)))
```

Other focused observations were:

- `ro mlatu cu jbena` produced a typed universal restriction with
  `(Import Projective)`;
- `ti blanu zdani` produced the tanru projection
  `((OfKind zdani blanu) This)` without a manufactured shared definition;
- `li pa su'i re du li ci` kept the proven `(+ 1 2)` math form and used complete
  local typed objects for the surrounding number descriptions it could not
  compact faithfully;
- names introduced by `la` remained compact inside `RespectivelyValue`;
- `ti mo zdani` retained typed `PredTerm` and `Relation` `Let` bindings, with
  every use lexically bound;
- quotation, hostile-string escaping, both respectively surfaces, termsets,
  complex connectives, displayed content, `zi'o`, `ni'o mi klama`, and
  `ti mo zdani` all built and rendered in the focused slice without TypedGraph
  or panic;
- the structural word-card test observed exactly one `(Smusni 0 ...)` document
  with `(Words ...)` inside it and exactly one trailing newline.

## Reproducibility and interpretation

The report program writes no expectations. Re-running a slice emits aggregate
tab-separated `SUMMARY`, `BUILD_FAILURE`, and `FALLBACK_REASON` records. The
structural test suite independently checks balanced parsing, deterministic
rendering, lexical `$` binding, graph `@` resolution, diagnostic multiplicity,
modal place maps, typed fallback fields, word-card integration, and the absence
of retired flat-smusni markers.
