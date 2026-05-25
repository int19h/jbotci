### !!! IMPORTANT !!!
Always keep in mind: our primary goals are correctness and code quality. If something is badly written and needs to be refactored, do it even if that means code churn; we have tests precisely so that we can refactor boldly to maintain high quality and clarity of the code. And correctness is always an absolute goal that is above everything else, so never take any shortcuts or skip verification because it's more convenient to just assume that something is now correct; always verify, and be skeptical and thorough.

# Project repo

jbotci ("Lojban tool") is intended to be a "swiss army knife" of Lojban in a single self-contained binary. Thus we want to compile it as a static no-deps binary for Linux, and as close as we can get to that for macOS and Windows (the old repo has that for Haskell, you can use it as a baseline but bear in mind that some things are the way they are because of Haskell toolchain limitations that may not apply to Rust so always think about how idiomatic Rust would approach the same problem first). We will eventually use Dioxus for the web part but the initial goal is to get CLI fully functional.

The project is hosted at https://codeberg.org/int_19h/jbotci/

Use token in ~/git/.codeberg/jbotci.token to access it using forjego-cli to browse or create issues.


# Porting guide

We're going to be working on jbotci v1.

You can find jbotci v0 in ~/git/jbotci.v0. It is written in Haskell. There's ~/git/jbotci.v0/AGENTS.md that describes some of the things in that repo, but note that this file is *not* to be treated as your guidance, only as reference material. Only the AGENTS.md in this repo is your guidance when working on things in this repo.

jbotci v1 is aiming to be a Rust port of everything that is in jbotci v0.

Our end goal is full feature parity, but we will build it up gradually, although accounting for future requirements when designing current architecture (meaning that e.g. the core libraries should account for being used in a wasm environment for a web SPA in the future).

Unlike the Haskell codebase, we want to separate the CLI app from the web app (the latter including API endpoints for MCP and Discord). jbotci will be the CLI app, and jbotci-server will be the web app. Shared code - parser, semantics etc - will be in shared crates.

We also eventually want to package jbotci as a pure GUI app for iOS, Android, macOS, and Linux. Dioxus should take care of most of this, but do bear this in mind when it comes to repo organization.

~/git/jbotci.0 is your own private copy of the original codebase so you can go wild there and change the code as you see fit as part of the porting work, e.g. to add the test export scripts. It's already on a separate branch so that whatever you do, you can always just revert to main or compare to it. 


# Coding style

In general: code quality matters. Avoid hacky solutions and don't ignore issues by claiming that they are "corner cases". A corner case is no less valuable, and a bug is a bug. Layering workarounds on top of broken code leads to more bugs so don't do that! If you have a choice between a major refactor that will do the Right Thing, and a small change that's patching over the problem or solving it in a hacky way, prefer the major refactor. Be aggressive about removing unused code. Make sure that your comments provide sufficient context as to _why_ something non-obvious is done the way it is, not just _what_ it does.

Never add heuristics anywhere unless they have been explicitly discussed with the user first and approved for that specific case. Heuristics are not a valid default way to handle unforeseen problems; if a heuristic is truly unavoidable, treat that as an exceptional compromise that must be called out and approved in advance.

When choosing between a narrow targeted fix and a broader correctness-first fix, prefer the broader "Right Thing" fix whenever it materially improves correctness.

When a refactor reaches the point where only larger cross-cutting slices will remove the real bottlenecks, prefer the larger correctness-first conversion over continuing to hunt for tiny isolated edits. Start from a green baseline, make the broader change, and use the full test suite as the regression oracle.

Use strong typing to your advantage. Prefer approaches that guarantee correctness by construction: for example, prefer strongly typed data where types capture constraints and invariants as much as possible over ad hoc stringing together of things. Use typeclasses judiciously to extract common features and enable their use without duplication. Avoid making untyped blobs by putting data into strings etc that have internal structure that is not enforced by the type system.

Prefer lifetime management techniques that can be statically enforced and are correct-by-construction. If possible, work with the borrow checker. If the data structure is not amenable to that, but refcounting solves it, use refcounting, but try to avoid weak references, indices into separately stored collections etc, since these are all prone to bugs such as dangling references or sudden state invalidation. 

Avoid unnecessary copies, clones, and temporary collections, especially in parser, morphology, AST traversal, and contract code. Prefer borrowing for read-only checks, consuming APIs for owned transformations, and in-place mutation when a caller already owns the collection. When both borrowed and owned use cases exist, split helpers into borrowed forms such as `as_...`, `..._ref`, or visitor/count helpers, and owned forms such as `into_...` or `append_..._to` that move values into their destination. Prefer extending a caller-provided accumulator over returning a temporary `Vec` that is immediately consumed, and prefer direct comparisons or small structural helpers over allocating `String`s only to test equality.

For the parser, keep struct fields ordered the same way the constructs appear in the input stream, and ensure pretty-printed outputs preserve that order.

Prefer structs over tuples, including in ADT constructors. If constructor wraps more than one value, it should have named fields.

Use descriptive commit messages that clearly summarize the change batch.

Commit periodically at green checkpoints during substantial refactors so progress is preserved in reviewable batches.

Commit periodically in well-defined logical units while working, not only at the end.

Before reverting any commit, always inspect it carefully (`git show` + surrounding history), verify the commit message and nature of changes, and only revert after explicit reasoning confirms the revert is correct.

When working on a Codeberg work item, assign it to yourself, and reference it in your commit message so that it is properly linked. If your commit _fully_ resolves the issue, then - and only then - reference the work item in such a way that it is automatically closed.

For corpus Lean typecheck failures, do not assume the renderer is wrong by default: inspect the original Lojban carefully and decide whether the corpus example is semantically/type-correct or whether the corpus itself contains a genuinely ill-typed example.

For real semantic divergences in Lean output, investigate them carefully before changing expectations: consult the relevant CLL section, use jbotci MCP and other reference materials to understand the example.

When intended behavior is unclear or a semantic question is in doubt, use jbotci cukta MCP to consult the CLL and clarify the intended reading before deciding on a fix or expectation change.

Be conservative about adding AST-comparison normalizations for Lean expectations: every such normalization can hide bugs. If only a small number of cases differ, prefer updating the affected expectations after careful semantic verification instead of teaching the comparer to treat the outputs as equivalent.

If you add debug logging that is broadly useful beyond a one-off investigation, gate it behind an environment variable and document what it traces, how to enable it, and when it is useful. Do not leave ad hoc always-on debug output in the tree.


# Design by contract

Use `bityzba` for design by contract throughout the workspace, including private functions, trait methods, `impl` methods, and public model types. `bityzba` is the public facade crate; `bityzba-macros` is an implementation detail and should not be imported directly. Import the macros you need from `bityzba::{requires, ensures, invariant, contract_trait, expensive_requires, expensive_ensures, expensive_invariant, data, new, try_new}`.

Cheap contracts use `requires`, `ensures`, and `invariant`; they run in normal builds and should be cheap enough for routine execution. Expensive contracts use `expensive_requires`, `expensive_ensures`, and `expensive_invariant`; they are disabled by default and enabled by `cargo test --workspace --all-targets --features expensive_contracts -j 16 -- --test-threads=16`. Do not use `test_requires`, `test_ensures`, or `test_invariant` for production expensive contracts; reserve them for genuinely test-only APIs.

Keep contracts in mind whenever writing or touching code. Capture preconditions, postconditions, type invariants, and function or `impl` invariants where they make correctness assumptions explicit. The build-time contract scanner requires every function and method to have both a precondition marker and a postcondition marker, and every struct or enum to have an invariant marker. Reason about the real contract first. Use `#[requires(true)]`, `#[ensures(true)]`, or `#[invariant(true)]` only when there is genuinely no stronger useful contract beyond what the types already express. The order should always be: requires, ensures, invariant.

Prefer correctness by construction over downstream validity checks: if a public model type has an invariant, put `#[invariant]` on the type and construct it through generated validation APIs instead of exposing public `is_valid()`. On data types, `#[invariant(true)]` and `#[expensive_invariant(true)]` are explicit audited no-op markers and do not generate wrapper/data APIs.

```rust
use bityzba::{
    contract_trait, data, ensures, expensive_ensures, expensive_requires, invariant, new,
    requires, try_new,
};

#[contract_trait]
trait RandomSource {
    #[requires(min < max)]
    #[ensures(min <= ret && ret <= max)]
    #[expensive_ensures(samples_are_uniform(ret, min, max))]
    fn gen(&self, min: f64, max: f64) -> f64;

    #[expensive_requires(weights.iter().all(|weight| *weight > 0.0))]
    #[ensures(true)]
    fn choose(&self, weights: &[f64]) -> usize;
}

#[contract_trait]
impl RandomSource for AlwaysMax {
    fn gen(&self, _min: f64, max: f64) -> f64 {
        max
    }

    fn choose(&self, _weights: &[f64]) -> usize {
        0
    }
}

#[requires(true)]
#[ensures(*x == old(*x) + 1, "after the call `x` was incremented")]
fn incr(x: &mut usize) {
    *x += 1;
}

#[requires(true)]
#[ensures(person_name.is_some() -> ret.contains(person_name.unwrap()))]
fn greeting(person_name: Option<&str>) -> String {
    let mut s = String::from("Hello");
    if let Some(name) = person_name {
        s.push(' ');
        s.push_str(name);
    }
    s.push('!');
    s
}

#[invariant(self.byte_start <= self.byte_end, "byte range must be ordered")]
#[invariant(self.char_start <= self.char_end, "character range must be ordered")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSpan {
    pub source_id: Option<SourceId>,
    pub byte_start: usize,
    pub byte_end: usize,
    pub char_start: usize,
    pub char_end: usize,
    pub start: Option<LineColumn>,
    pub end: Option<LineColumn>,
}

impl SourceSpan {
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|span| span.byte_start == byte_start))]
    pub fn new(
        source_id: Option<SourceId>,
        byte_start: usize,
        byte_end: usize,
        char_start: usize,
        char_end: usize,
    ) -> Result<Self, SourceLocationError> {
        if byte_end < byte_start {
            return Err(SourceLocationError::ByteRangeInverted {
                start: byte_start,
                end: byte_end,
            });
        }
        if char_end < char_start {
            return Err(SourceLocationError::CharRangeInverted {
                start: char_start,
                end: char_end,
            });
        }
        Ok(Self::from_data(data!(SourceSpan {
            source_id: source_id,
            byte_start: byte_start,
            byte_end: byte_end,
            char_start: char_start,
            char_end: char_end,
            start: None,
            end: None,
        }))
    }
}

#[invariant(::Pair => opt_gihi(gihi) && vau.is_absent_or_cmavo(Cmavo::Vau))]
#[invariant(::Ke => ke.is_cmavo(Cmavo::Ke) && kehe.is_absent_or_cmavo(Cmavo::Kehe))]
#[invariant(::Na => na.is_selmaho(Selmaho::Na))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum GekSentenceSyntax {
    Pair {
        gek: ConnectiveSyntax,
        first: Box<SubsentenceSyntax>,
        gik: ConnectiveSyntax,
        second: Box<SubsentenceSyntax>,
        gihi: Option<WithIndicators<WordLike>>,
        tail_terms: Vec<TermSyntax>,
        vau: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Ke {
        tense_modal: Option<Box<TenseModalSyntax>>,
        ke: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner: Box<GekSentenceSyntax>,
        kehe: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Na {
        na: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner: Box<GekSentenceSyntax>,
    },
}
```

For type invariants, `#[invariant]` on a named-field struct or enum creates a validated wrapper plus an unchecked `TypeData`. Values of the wrapper are valid by construction. Do not add public `is_valid()` to invariant-bearing model types. Keep private helper predicates only for smaller sub-concepts that are not themselves represented by validated types.

Use `new!(Type { ... })` for normal full construction and `try_new!(Type { ... })` at fallible unchecked boundaries. Every field must be present exactly once; missing fields fail at compile time through builder typestate and duplicate fields are a macro error. Use `value.with_data(data! { ... })` for whole-value updates; it consumes the old value, applies the partial field set, and revalidates. Clone first if the old value must be retained. `bityzba` does not generate `Type::new`; if a type needs a hand-written constructor that does real work beyond invariant validation, implement it in terms of `from_data` or `try_from_data`.

```rust
let span = new!(CheckedSpan {
    source_id: None,
    byte_start: 0,
    byte_end: 12,
    char_start: 0,
    char_end: 12,
});

let span = try_new!(CheckedSpan {
    source_id: None,
    byte_start: 0,
    byte_end: 12,
    char_start: 0,
    char_end: 12,
})?;

let span = span.with_data(data! {
    byte_end: 16,
    char_end: 16,
});

assert_eq!(span.byte_end, 16); // read-only field access through Deref
```

Enums use `new!` with the normal variant shape: named variants use braces, tuple variants use parentheses, and unit variants use a path.

```rust
let value = new!(SyntaxValue::Node { node });
let value = new!(SyntaxValue::Null);
let value = new!(Example::Pair(left, right));
```

There is no generated `DerefMut`. Do not mutate fields directly. If low-level code must work with unchecked data, use `value.as_data()`, `value.into_data()`, `Type::try_from_data(data)`, `Type::from_data(data)`, or `TryFrom<TypeData>` explicitly and keep that escape hatch local.

Use `data!` pattern aliases when destructuring data views so normal code does not mention data type names:

```rust
let data!(SourceSpan { byte_start, byte_end, .. }) = span.as_data();

match value.as_data() {
    data!(SyntaxValue::Node { node }) => visit(node),
    data!(SyntaxValue::Word { word }) => visit_word(word),
    data!(Example::Pair(left, right)) => visit_pair(left, right),
    _ => {}
}
```

Use the ownership-preserving bityzba API that matches the job. Use `as_data()` and `data!` patterns for read-only inspection; use `into_data()` when the wrapper is already owned and fields should be moved instead of cloned. For whole-value updates where the replacement is already available, use `value.with_data(data! { field: replacement })`. For updates that derive a new field by consuming the old field, move to data and use normal struct update syntax through `data!`, including `..data`, then revalidate with `Type::from_data` or `Type::try_from_data`:

```rust
let data = word.into_data();
let word = Word::from_data(data!(Word {
    span: map_span(data.span)?,
    ..data
}));
```

`data!(Type { ... })` and `TypeData` are unchecked escape hatches for serde internals, low-level tests, fixtures, and rare advanced code. They obey normal Rust privacy; use `new!` for public construction of structs with private fields.

Use cheap contracts for local scalar checks, shape checks already needed by callers, and invariants that are constant-time or close to it. Use expensive contracts for corpus-wide validation, deep tree walks, normalization cross-checks, semantic equivalence checks, and any contract that allocates, traverses large collections, calls parsers, or performs work that would be inappropriate in normal builds.

Avoid an anti-pattern where the entire contract is tucked away in a separately declared function. Contracts should be preferentially attached directly to the function or type that they constrain, so that when you scan the code you immediately see the gist of the contract. For example, this contract is opaque since one needs to find and read `is_valid_phonemes_text` to understand what it does:
```
#[invariant(is_valid_phonemes_text(&self.text), "phonemes must be canonical Lojban phoneme text")]
```
a better way to write it is:
```
#[invariant(!text.is_empty() && text.chars().all(|phoneme| { ... })]
```
Here it is immediately clear when looking at the definition of the struct that the text is never empty and what values it can take. Helper functions that exist *solely* to be called from a single contract assertion are thus a code smell. It is okay to define such helper functions only when they are reused, when they need to recurse, or when the function is so complex that its definition would span more than several dozen lines. 

All non-bityzba workspace crates run `bityzba::require_contracts().unwrap()` from `build.rs` with the `contract_scanner` feature enabled. The scanner is syntactic: it checks explicit source attributes under `src`, `tests`, `benches`, `examples`, and `build.rs`; it does not inspect macro expansions and does not count contracts hidden inside `cfg_attr`.


# Build

Always build the `jbotci` release binary before you wrap up your work.


# Test suite

In all cases, when updating expectations, be VERY careful and do not blindly assume that new output is correct. Ideally expectations need to be updated manually rather than taking the output of the parser. When that is infeasible (e.g. you have more than a few hundred expectations that need updating), you need to definitively ascertain that the change is purely surface representation before doing any kind of automatic expectation update whatsoever. That is, if the change is predictable and mechanical, write a script that validates this while comparing the old and the new expectations, and only for those entries where such a script confirms that it is indeed only that mechanical change that is the difference, it is okay to update with new output; any cases that get flagged as more than that need careful *manual* review individually, one by one.

Prefer to run tests on release builds - the test suite is large and parser performance is degraded substantially in debug builds, so even with the cost of rebuild, release builds are still several times faster. Use debug builds only when it is necessary to debug test behavior.


# CLL Errata: Commas and Glides

BPFK morphology treats commas as syllable separators only; commas do not affect glide/hiatus detection.

The CgV (consonant-glide-vowel) ban applies across commas, so names that rely on a comma to block a glide are invalid (e.g., `.an,iis.`, `.melxi,or.`).

The semantics of `le` and `lo` and default quantification rules are different. If you have read CLL regarding either of these as part of your research, you must also check https://mw.lojban.org/papri/How_to_use_xorlo for an important clarification before using the information from CLL for any decisions pertaining to Lojban semantics.


# Lojban EBNF Grammar

Explanation of notation: All rules have the form:
```
name = bnf-expression
```
which means that the grammatical construct “name” is defined by “bnf-expression”.

Names in lower case are grammatical constructs.

Names in UPPER CASE are selma'o (lexeme) names, and are terminals.

Concatenation is expressed by juxtaposition with no operator symbol.

`|` represents alternation (choice).

`[]` represents an optional element.

`&` represents and/or. `A & B` is the same as `A | B | A B` but not `B A`. Furthermore, `A & B & C & D` permits one or more of A, B, C, and/or D, but only in that order.

`...` represents optional repetition of the construct to the left. Left-grouping is implied; right-grouping is shown by explicit self-referential recursion with no `...`

`()` serves to indicate the grouping of the other operators. Otherwise, `...` binds closer than `&`, which binds closer than `|`.

`#` is shorthand for `[free ...]` , a construct which appears in many places.

`//` encloses an elidable terminator, which may be omitted (without change of meaning) if no grammatical ambiguity results.

```ebnf
text =
[NAI ...] [CMEVLA ... # | (indicators & free ...)] [joik-jek] text-1

text-1 =
[(I [jek | joik] [[stag] BO] #) ... | NIhO ... #] [paragraphs]

paragraphs =
paragraph [NIhO ... # paragraphs]

paragraph =
(statement | fragment) [I # [statement | fragment]] ...

statement =
statement-1 | prenex statement

statement-1 =
statement-2 [I joik-jek [statement-2]] ...

statement-2 =
statement-3 [I [jek | joik] [stag] BO # [statement-2]]

statement-3 =
sentence | [tag] TUhE # text-1 /TUhU#/

fragment =
ek # | gihek # | quantifier | NA # | terms /VAU#/ | prenex | relative-clauses | links | linkargs

prenex =
terms ZOhU #

sentence =
[terms [CU #]] bridi-tail

subsentence =
sentence | prenex subsentence

bridi-tail =
bridi-tail-1 [gihek [stag] KE # bridi-tail /KEhE#/ tail-terms]

bridi-tail-1 =
bridi-tail-2 [gihek # bridi-tail-2 tail-terms] ...

bridi-tail-2 =
bridi-tail-3 [gihek [stag] BO # bridi-tail-2 tail-terms]

bridi-tail-3 =
selbri tail-terms | gek-sentence

gek-sentence =
gek subsentence gik subsentence tail-terms | [tag] KE # gek-sentence /KEhE#/ | NA # gek-sentence

tail-terms =
[terms] /VAU#/

terms =
terms-1 ...

terms-1 =
terms-2 [PEhE # joik-jek terms-2] ...

terms-2 =
term [CEhE # term] ...

term =
sumti | (tag | FA #) (sumti | /KU#/) | termset | NA KU #

termset =
NUhI # gek terms /NUhU#/ gik terms /NUhU#/ | NUhI # terms /NUhU#/

sumti =
sumti-1 [VUhO # relative-clauses]

sumti-1 =
sumti-2 [(ek | joik) [stag] KE # sumti /KEhE#/]

sumti-2 =
sumti-3 [joik-ek sumti-3] ...

sumti-3 =
sumti-4 [(ek | joik) [stag] BO # sumti-3]

sumti-4 =
sumti-5 | gek sumti gik sumti-4

sumti-5 =
[quantifier] sumti-6 [relative-clauses] | quantifier selbri /KU#/ [relative-clauses]

sumti-6 =
(LAhE # | NAhE BO #) [relative-clauses] sumti /LUhU#/ | KOhA # | lerfu-string /BOI#/ | LA # [relative-clauses] CMEVLA ... # | (LA | LE) # sumti-tail /KU#/ | LI # mex /LOhO#/ | ZO any-word # | LU text /LIhU#/ | LOhU any-word ... LEhU # | ZOI any-word anything any-word #

sumti-tail =
[sumti-6 [relative-clauses]] sumti-tail-1 | relative-clauses sumti-tail-1

sumti-tail-1 =
[quantifier] selbri [relative-clauses] | quantifier sumti

relative-clauses =
relative-clause [ZIhE # relative-clause] ...

relative-clause =
GOI # term /GEhU#/ | NOI # subsentence /KUhO#/

selbri =
[tag] selbri-1

selbri-1 =
selbri-2 | NA # selbri

selbri-2 =
selbri-3 [CO # selbri-2]

selbri-3 =
selbri-4 ...

selbri-4 =
selbri-5 [joik-jek selbri-5 | joik [stag] KE # selbri-3 /KEhE#/] ...

selbri-5 =
selbri-6 [(jek | joik) [stag] BO # selbri-5]

selbri-6 =
tanru-unit [BO # selbri-6] | [NAhE #] guhek selbri gik selbri-6

tanru-unit =
tanru-unit-1 [CEI # tanru-unit-1] ...

tanru-unit-1 =
tanru-unit-2 [linkargs]

tanru-unit-2 =
BRIVLA # | GOhA [RAhO] # | KE # selbri-3 /KEhE#/ | ME # sumti /MEhU#/ [MOI #] | (number | lerfu-string) MOI # | NUhA # mex-operator | SE # tanru-unit-2 | JAI # [tag] tanru-unit-2 | any-word (ZEI any-word) ... | NAhE # tanru-unit-2 | NU [NAI] # [joik-jek NU [NAI] #] ... subsentence /KEI#/

linkargs =
BE # term [links] /BEhO#/

links =
BEI # term [links]

quantifier =
number /BOI#/ | VEI # mex /VEhO#/

mex =
mex-1 [operator mex-1] ... | FUhA # rp-expression

mex-1 =
mex-2 [BIhE # operator mex-1]

mex-2 =
operand | [PEhO #] operator mex-2 ... /KUhE#/

rp-expression =
rp-operand rp-operand operator

rp-operand =
operand | rp-expression

operator =
operator-1 [joik-jek operator-1 | joik [stag] KE # operator /KEhE#/] ...

operator-1 =
operator-2 | guhek operator-1 gik operator-2 | operator-2 (jek | joik) [stag] BO # operator-1

operator-2 =
mex-operator | KE # operator /KEhE#/

mex-operator =
SE # mex-operator | NAhE # mex-operator | MAhO # mex /TEhU#/ | NAhU # selbri /TEhU#/ | VUhU #

operand =
operand-1 [(ek | joik) [stag] KE # operand /KEhE#/]

operand-1 =
operand-2 [joik-ek operand-2] ...

operand-2 =
operand-3 [(ek | joik) [stag] BO # operand-2]

operand-3 =
quantifier | lerfu-string /BOI#/ | NIhE # selbri /TEhU#/ | MOhE # sumti /TEhU#/ | JOhI # mex-2 ... /TEhU#/ | gek operand gik operand-3 | (LAhE # | NAhE BO #) operand /LUhU#/

number =
PA [PA | lerfu-word] ...

lerfu-string =
lerfu-word [PA | lerfu-word] ...

lerfu-word =
BY | any-word BU | LAU lerfu-word | TEI lerfu-string FOI

ek =
[NA] [SE] A [NAI]

gihek =
[NA] [SE] GIhA [NAI]

jek =
[NA] [SE] JA [NAI]

joik =
[SE] JOI [NAI] | interval | GAhO interval GAhO

interval =
[SE] BIhI [NAI]

joik-ek =
joik # | ek #

joik-jek =
joik # | jek #

gek =
[SE] GA [NAI] # | joik GI # | stag gik

guhek =
[SE] GUhA [NAI] #

gik =
GI [NAI] #

tag =
tense-modal [joik-jek tense-modal] ...

stag =
simple-tense-modal [(jek | joik) simple-tense-modal] ...

tense-modal =
simple-tense-modal # | FIhO # selbri /FEhU#/

simple-tense-modal =
[NAhE] [SE] BAI [NAI] [KI] | [NAhE] (time [space] | space [time]) & CAhA [KI] | KI | CUhE

time =
ZI & time-offset ... & (ZEhA [PU [NAI]]) & interval-property ...

time-offset =
PU [NAI] [ZI]

space =
VA & space-offset ... & space-interval & (MOhI space-offset)

space-offset =
FAhA [NAI] [VA]

space-interval =
((VEhA & VIhA) [FAhA [NAI]]) & space-int-props

space-int-props =
(FEhE interval-property) ...

interval-property =
number ROI [NAI] | TAhE [NAI] | ZAhO [NAI]

free =
SEI # [terms [CU #]] selbri /SEhU/ | SOI # sumti [sumti] /SEhU/ | vocative [relative-clauses] selbri [relative-clauses] /DOhU/ | vocative [relative-clauses] CMEVLA ... # [relative-clauses] /DOhU/ | vocative [sumti] /DOhU/ | (number | lerfu-string) MAI | TO text /TOI/ | XI # (number | lerfu-string) /BOI/ | XI # VEI # mex /VEhO/

vocative =
(COI [NAI]) ... & DOI

indicators =
[FUhE] indicator ...

indicator =
(UI | CAI) [NAI] | Y | DAhO | FUhO

The following rules are non-formal:

word =
[BAhE] any-word [indicators]

any-word =
“any single word (no compound cmavo)”

anything =
“any text at all, whether Lojban or not”

null =
any-word SI | utterance SA | text SU
```
