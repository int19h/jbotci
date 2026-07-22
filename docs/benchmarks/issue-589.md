# Issue #589 phonetic gimfi'i benchmark

Measured on 2026-07-22 on Linux/aarch64 with Rust 1.96.1. The host exposed 15
single-threaded logical CPUs, but the scorer and benchmark ran serially.

## Method

The baseline was a detached worktree at frozen commit
`e3e0396521bac46a91ba1c2dc68e89aba905c42b`. Before building it:

- `crates/jbotci-gimfihi/src/lib.rs` and
  `crates/jbotci-phonetic/src/lib.rs` were verified unchanged from that commit;
- the only untracked path was the benchmark example; and
- the baseline and optimized benchmark harnesses had the same SHA-256,
  `ebe0bb84c08bdda1debb86c6b8ce591917ee270580800b6c63fb64ac57ac4eb9`.

The exact commands, run sequentially, were:

```console
# /tmp/jbotci589-base.FeA3uv
CARGO_TARGET_DIR=/home/int19h.linux/git/.jbotci-589-base-target cargo build -r -p jbotci-gimfihi --example issue_587_benchmark
/home/int19h.linux/git/.jbotci-589-base-target/release/examples/issue_587_benchmark 10

# /home/int19h.linux/git/jbotci-issue-589
cargo build -r -p jbotci-gimfihi --example issue_587_benchmark
target/release/examples/issue_587_benchmark 10
```

The example performs one unmeasured warm-up, then 10 measured executions of
the exact #587 request. The median is the mean of the two middle samples; the
mean includes all 10 samples. Output hashing and assertions happen outside the
timed interval.

## Results

| Revision | Median | Mean |
|---|---:|---:|
| Frozen pre-change scorer | 926.186 ms | 930.714 ms |
| Precomputed scorer and bounded selection | 731.519 ms | 741.345 ms |

The optimized median was 21.018% lower (1.266x throughput); the mean was
20.347% lower (1.255x throughput). These are single-host wall-clock results,
so the reproducible commands and semantic parity are more important than the
specific ratio.

## Output parity

Both binaries produced the same FNV-1a signature over the complete returned
`GimfihiOutput` debug projection: `3365e6aa5e35ac4a`. Both also reported:

- `candidate_count = 96,475`;
- `filtered_count = 82,567`;
- winner `faxne`; and
- identical top score bits:
  `faxne = 4601754544623881066` (`0.44869245574425654`),
  `faxme = 4601737933020898136` (`0.4477703265388775`), and
  `fexne = 4601719064802835984` (`0.446722930032565`).

The benchmark also asserts that every measured execution matches its warm-up
signature. Unit tests independently compare concrete eager scoring against the
optimized output bit-for-bit across every normalizer and compare prepared and
concrete alignment for all 96,475 valid gismu candidates.

## Disk usage

Disk readings used `df -B1` immediately before the baseline build and after
both measurements:

| Mount | Used before | Used after | Delta |
|---|---:|---:|---:|
| `/` | 181,541,015,552 B | 181,545,226,240 B | +4,210,688 B |
| `/home/int19h.linux/git` | 957,278,568,448 B | 957,651,329,024 B | +372,760,576 B |
| `/tmp` | 2,928,664,576 B | 2,928,664,576 B | 0 B |

The mounted-disk increase is primarily the isolated 378 MiB baseline Cargo
target. It was retained through verification so the recorded baseline remains
inspectable.
