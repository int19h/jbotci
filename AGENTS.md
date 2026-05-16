# Project repo

The project is hosted at https://codeberg.org/int_19h/jbotci/

Use token in ~/git/.codeberg/jbotci.token to access it using forjego-cli to browse or create issues.


# Coding style

In general: code quality matters. Avoid hacky solutions and don't ignore issues by claiming that they are "corner cases". A corner case is no less valuable, and a bug is a bug. Layering workarounds on top of broken code leads to more bugs so don't do that! If you have a choice between a major refactor that will do the Right Thing, and a small change that's patching over the problem or solving it in a hacky way, prefer the major refactor. Be aggressive about removing unused code. Make sure that your comments provide sufficient context as to _why_ something non-obvious is done the way it is, not just _what_ it does.

Never add heuristics anywhere unless they have been explicitly discussed with the user first and approved for that specific case. Heuristics are not a valid default way to handle unforeseen problems; if a heuristic is truly unavoidable, treat that as an exceptional compromise that must be called out and approved in advance.

When choosing between a narrow targeted fix and a broader correctness-first fix, prefer the broader "Right Thing" fix whenever it materially improves correctness.

When a refactor reaches the point where only larger cross-cutting slices will remove the real bottlenecks, prefer the larger correctness-first conversion over continuing to hunt for tiny isolated edits. Start from a green baseline, make the broader change, and use the full test suite as the regression oracle.

Use strong typing to your advantage. Prefer approaches that guarantee correctness by construction: for example, prefer strongly typed data where types capture constraints and invariants as much as possible over ad hoc stringing together of things. Use typeclasses judiciously to extract common features and enable their use without duplication. Avoid making untyped blobs by putting data into strings etc that have internal structure that is not enforced by the type system.

Prefer lifetime management techniques that can be statically enforced and are correct-by-construction. If possible, work with the borrow checker. If the data structure is not amenable to that, but refcounting solves it, use refcounting, but try to avoid weak references, indices into separately stored collections etc, since these are all prone to bugs such as dangling references or sudden state invalidation. 

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

Use design by contract for all code, including private members and types, and methods of traits and `impl`. DbC crate is https://github.com/x52dev/contracts. Capture all preconditions and postconditions, even those that are expensive to validate. For expensive checks, use `test_requires`, `test_ensures`, and `test_invariant`, otherwise use `requires`, `ensures`, and `invariant`. Examples:

Keep contracts in mind whenever writing or touching code: define data-type invariants, function preconditions and postconditions, and function or `impl` invariants wherever they make correctness assumptions explicit.

```rust
#[contract_trait]
trait MyRandom {
    #[requires(min < max)]
    #[ensures(min <= ret, ret <= max)]
    fn gen(min: f64, max: f64) -> f64;
}

#[contract_trait]
impl MyRandom for AlwaysMax {
    fn gen(min: f64, max: f64) -> f64 {
        max
    }
}

#[ensures(*x == old(*x) + 1, "after the call `x` was incremented")]
fn incr(x: &mut usize) {
    *x += 1;
}

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
```


# CLL Errata: Commas and Glides

BPFK morphology treats commas as syllable separators only; commas do not affect glide/hiatus detection.

The CgV (consonant-glide-vowel) ban applies across commas, so names that rely on a comma to block a glide are invalid (e.g., `.an,iis.`, `.melxi,or.`).
