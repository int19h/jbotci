# Test Fixtures

The v1 test suite uses one TOML file per test case. A fixture keeps source text,
provenance, and every expectation for that case in one place.

Fixture files live under `fixtures/`, organized by provenance:

```text
fixtures/
  cll/chapter-18/section-18.3/c18e3d1.toml
  muplis/collection-18/<case>.toml
  corpus/camxes/<case>.toml
  adhoc/<topic>/<case>.toml
```

The layout is part of the developer interface: a failing CLL test should point
to a fixture whose path and provenance fields make the original CLL context
easy to inspect.

## Common Shape

```toml
id = "cll.18.3.c18e3d1"
lojban = "..."
translation-en = "..."
gloss-en = "..."
tags = ["cll", "chapter-18", "syntax"]

[[provenance]]
kind = "cll"
chapter = 18
section-number = "18.3"
section-id = "c18s3"
example-number = "18.12"
example-id = "c18e3d1"
source-path = "vendor/cll/chapters/18.xml"

[expectations.morphology]
status = "success"

[[expectations.morphology.words]]
kind = "base-word"

[expectations.morphology.words.word_like]
kind = "bare"

[expectations.morphology.words.word_like.word]
kind = "cmavo"
phonemes = "coi"
surface_override = "coi"

[expectations.morphology.words.word_like.word.span]
source_id = "<fixture>"
byte_start = 0
byte_end = 3
char_start = 0
char_end = 3

[expectations.syntax]
status = "success"
parse-tree = {
    kind = "node",
    node = {
        constructor = "LojbanText",
        fields = []
    }
}
```

Every facet is optional so exporters can land expectations incrementally. The
long-term goal is one uniform fixture format for CLL, muplis, camxes corpus,
and ad hoc regression cases. Test runners should allow selection by both
fixture groups and facet, for example all CLL chapter 18 syntax tests or all
CLL plus muplis semantics tests.

Profiles live under `fixtures/profiles/` and combine facet selection with the
same selectors accepted by `cargo xtask fixture-list` and `cargo xtask
fixture-test`. The `cargo` profile intentionally selects no facets so ordinary
`cargo test` can validate loading and filtering without running unported parser
facets.
