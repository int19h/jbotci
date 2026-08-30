# Test Fixtures

The v1 test suite uses one TOML file per test case. A fixture keeps source text,
provenance, and every expectation for that case in one place.

Fixture files live under `tests/fixtures/`, organized by provenance:

```text
tests/fixtures/
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
tags = ["regression"]

[[provenance]]
kind = "cll"
chapter = 18
section-number = "18.3"
section-id = "c18s3"
example-number = "18.12"
example-id = "c18e3d1"
source-path = "vendor/cll/chapters/18.xml"

[expectations.output]
brackets = "[coi]"

[expectations.morphology]
status = "success"
words = [
    {Bare = {
        kind = "cmavo",
        phonemes = "coi",
        span = [0, 3]
    }},
]

[expectations.syntax]
status = "success"
parse-tree = {}
```

Large fixtures may keep the source text in a sibling file instead of embedding
it directly in TOML:

```toml
id = "cll.chrestomathy.north-wind"
lojban-filename = "texts/north-wind.lojban"
tags = ["long-text", "regression-baseline"]
```

`lojban-filename` is relative to the fixture TOML file. It is mutually
exclusive with inline `lojban`; the loader resolves it into `TestCase.lojban`
before any runner sees the fixture.

Every facet is optional so exporters can land expectations incrementally. The
long-term goal is one uniform fixture format for CLL, muplis, camxes corpus,
and ad hoc regression cases. Test runners should allow selection by both
fixture groups and facet, for example all CLL chapter 18 syntax tests or all
CLL plus muplis reference-analysis tests.

Tags are for ad hoc organization that is not already captured by provenance,
path, or structured selectors. For example, CLL chapter membership belongs in
`provenance`, not in `tags`.

Profiles live under `tests/fixtures/profiles/` and combine facet selection with the
same selectors accepted by `cargo xtask fixture-list` and `cargo xtask
fixture-test`. The `cargo` profile intentionally selects no facets so ordinary
`cargo test` can validate loading and filtering without running unported parser
facets.

## Long-Text Benchmarks

The `long-texts` profile selects whole-text CLL chrestomathy fixtures. Use it as
the common corpus for parser hot-path measurements:

```sh
cargo run -r -p xtask-full -- fixture-test --profile long-texts
cargo run -r -p xtask-full -- syntax-parser-benchmark --profile long-texts --iterations 5
target/release/jbotci --benchmark 5 vlasei --file tests/fixtures/cll/chrestomathy/texts/north-wind.lojban --turtai json --indent 0
target/release/jbotci --benchmark 5 gentufa --file tests/fixtures/cll/chrestomathy/texts/north-wind.lojban --turtai json --indent 0
```

For aggregate before/after evidence, also record the wall time for:

```sh
/usr/bin/time -f 'elapsed=%E user=%U sys=%S maxrss_kb=%M' cargo run -r -p xtask-full -- fixture-test --profile all
```
