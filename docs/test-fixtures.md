# Test Fixtures

The v1 test suite uses one TOML file per test case. A fixture keeps source text,
provenance, and every expectation for that case in one place.

Fixture files live under `tests/fixtures/`, organized by provenance:

```text
tests/fixtures/
  cll/chapter-18/section-18.3/c18e3d1.toml
  cll/chrestomathy/<case>.toml
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

### CLL Provenance and Book Divisions

The book has two kinds of top-level division, and CLL provenance names each the
way the book itself does.

A numbered chapter is named by `chapter` together with the `section-number` the
book prints:

```toml
[[provenance]]
kind = "cll"
chapter = 18
section-number = "18.3"
section-id = "c18s3"
```

An appendix has no number and no letter — the book's table of contents runs
1-N and then lists the appendices by title — so appendix provenance names the
appendix's stable division id instead, and carries no `section-number` at all.
The section is addressed by its `section-id`:

```toml
[[provenance]]
kind = "cll"
appendix = "volume-chrestomathy"
section-id = "section-north-wind"
source-path = "vendor/cll/chapters/a01.xml"
```

`chapter` and `appendix` are mutually exclusive and exactly one must be present;
`section-number` is present exactly when `chapter` is; the chapter number is
non-zero and every recorded identifier is non-empty.

Two different checks own the two different ways provenance can be wrong.

**Structure, at load time.** `Provenance` is an invariant-bearing type, so a
malformed block is rejected while it is being deserialized - by
`load_fixture_file`, `load_fixture_path`, `load_fixture_tree`,
`validate_fixture_tree` and the JSON `import_export_file` alike - and reported
as a `ParseToml`/`ParseJson` error naming the file. No `TestCase` can hold a
malformed provenance, so downstream code may rely on the shape.

**Staleness, in the audit.** Load-time validation is deliberately
*structural only*: it never consults the CLL. A block such as `chapter = 22` /
`section-number = "22.1"` on a chrestomathy section is perfectly well shaped -
it just names a division the book does not put that section in, and since
colojban chapter 22 is a real chapter. Only a site-aware check can see that, so
`cargo run -r -p xtask-full -- cll-fixture-metadata-audit` resolves each
fixture's section in the embedded CLL and reports any division or section-number
disagreement as a `provenance_mismatch` **error**. For division fields, one side
present and the other absent counts as disagreement rather than as a benign
missing value.

Appendix fixtures live under a division directory named by that id rather than
under `chapter-NN/`, except for the hand-placed chrestomathy long texts, which
keep their historical `cll/chrestomathy/` home.

Select fixtures by division with `--cll-chapter 18` or
`--cll-appendix volume-chrestomathy`.

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
