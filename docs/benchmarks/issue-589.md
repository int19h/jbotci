# Phonetic gimfihi benchmark

The `issue_587_benchmark` example measures the phonetic gismu scorer.
It uses a fixed request with twelve source languages.
It computes all candidates and records a signature of the returned output.

## Operating the benchmark

From the repository root, build the example in release mode:

```sh
CARGO_TARGET_DIR=/build/jbotci/target/gimfihi-benchmark cargo build -r -p jbotci-gimfihi --example issue_587_benchmark
/build/jbotci/target/gimfihi-benchmark/release/examples/issue_587_benchmark 10
```

The argument selects the number of measured executions.
The example does one warm-up before those executions.
It computes output signatures and assertions outside the timed interval.
Each measured output must match the warm-up signature.

## Comparing revisions

Use separate worktrees and build lanes for each revision.
Operate the same benchmark harness on the same host with the same arguments.
Record the exact commits, toolchain, host, and output signatures with the results.

The scorer tests compare prepared scoring with eager scoring for each normalizer.
They also compare prepared and concrete alignment over the candidate inventory.
Timing comparisons do not replace these correctness tests.
