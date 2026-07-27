# Python distribution artifacts

The Python artifact lane builds reviewable files only. It has no package-index
credentials, publication command, release tag, or attestation step. Every
wheel and source distribution is retained for 14 days as a GitHub Actions
artifact.

## Supported matrix

One `abi3-py311` wheel is built and logically reproduced for each target. The
same wheel is then installed and tested on the oldest supported CPython and
the newest version currently declared by the package:

| Target | GitHub build runner | Wheel policy | Test interpreters |
| --- | --- | --- | --- |
| Linux x86_64 | `ubuntu-24.04` | manylinux 2.28 | CPython 3.11 and 3.14 |
| Linux aarch64 | `ubuntu-24.04` cross-build | manylinux 2.28 | CPython 3.11 and 3.14 on `ubuntu-24.04-arm` |
| macOS x86_64 | `macos-15-intel` | PyPI-compatible macOS tag | CPython 3.11 and 3.14 |
| macOS arm64 | `macos-15` | PyPI-compatible macOS tag | CPython 3.11 and 3.14 |
| Windows x86_64 | `windows-2025` | `win_amd64` | CPython 3.11 and 3.14 |

PyO3's configured `abi3-py311` feature works on every listed target, so there
is no interpreter-specific fallback wheel. Free-threaded CPython uses a
different stable ABI and is not part of the issue #564 desktop matrix.

The workflow runs for pull requests and `main` pushes only when the Python
package, a Rust crate, the parity tool, a lock/toolchain file, or the workflow
itself changes. It also supports `workflow_dispatch`. This path policy avoids
spending roughly seventeen cross-platform jobs on documentation-only changes;
the tradeoff is that unrelated workflow or application-only changes do not
retest already unchanged Python artifacts.

## What the artifact tests prove

`tools/run_wheel_tests.py` creates a new virtual environment outside the
checkout, installs only the selected wheel and the test dependencies declared
in `pyproject.toml`, removes Python path overrides, and delegates to
`tools/run_installed_examples.py`. That existing #563 runner now performs the
shared installed-package checks before executing every public example:

- every public module imports from the installed wheel;
- `py.typed`, the composed native stub, and both generated syntax stubs exist;
- native, reference, and generated syntax runtime inventories match the
  installed stubs;
- version, pre-alpha, license, and README metadata match the project;
- the embedded English dictionary loads and answers a real lookup;
- the public examples execute dictionary, morphology, strict/recovered
  parsing, generated syntax matching, jvozba, place assignment, and reference
  analysis outside the source tree;
- the existing #563 strict consumer fixture and examples pass mypy;
- public examples neither import `_native` nor inspect `__file__` for
  repository-relative data.

`tools/python_artifacts.py` rejects unsafe archive paths, symlinks, build/cache
directories, dependency test corpora, secret-like files, checkout paths,
unexpected large members, and unexpected archive layouts. It also emits one
JSON size receipt per artifact.

## Source-distribution proof

Maturin first builds two raw sdists. Maturin already trims the workspace member
list to the binding dependency closure, but it retains unrelated root
`workspace.dependencies` and `patch.crates-io` path entries. The artifact tool
asserts and removes an explicit audited list of those dangling entries instead
of shipping their unrelated source trees, has Cargo prune the retained lockfile
to that exact closure, and normalizes tar/gzip metadata.
Inspection proves that every path reference left in each standalone manifest
has a packaged `Cargo.toml`. The workflow then compares every logical member
byte-for-byte and:

1. inspects and unpacks the first sdist under the runner's temporary directory;
2. installs the pinned Maturin builder and prefetches locked Cargo
   dependencies;
3. moves the entire original checkout into a denied directory, verifies that
   its manifest is unreadable and no manifest remains at the checkout path,
   then restores the checkout only after the isolated proof finishes;
4. builds two wheels from the unpacked sdist with Cargo offline in a fresh,
   sdist-only target directory (only the dependency registry is cached);
5. compares, inspects, installs, and runs the same artifact tests against the
   sdist-built wheel.

The embedded dictionary JSON and metadata live in
`crates/jbotci-dictionary-data/data`, their owning Cargo package. Builds
compile those checked-in inputs and cannot download dictionary data.
Dependency Cargo packages exclude tests, benches, examples, and UI fixtures
from the sdist closure. The small syntax binding-schema consumer remains
because it is itself a Cargo path dependency represented in the packaged
workspace. The generated 2.9 MB API-parity audit TSV is checked before
packaging but omitted from the buildable source artifact; it is review
evidence, not a build input.

Maturin's generated Rust SBOM is disabled in `pyproject.toml`: its timestamp
changes on every build and its component references contain the absolute
checkout path. `Cargo.lock` and `uv.lock` remain in the sdist as the
deterministic dependency records. Re-enable generated SBOMs only when the
upstream output can be made reproducible and path-neutral.

## Size ratchet

`artifact-policy.toml` records compressed and unpacked baselines per platform,
an allowed growth percentage, and an entry-count ceiling. Inspection fails
above those derived thresholds. A baseline update is not a routine snapshot
refresh: inspect the logical member diff, explain the increase in the pull
request, then record the reviewed artifact's actual sizes.

The acceptance job downloads all receipts, requires all build and test matrix
jobs to have succeeded, and publishes a Markdown size table. A green matrix
leg cannot be replaced by a skipped leg.

## Owner reproduction

Start in `bindings/python` and keep all transient output outside the checkout:

```console
export CARGO_TARGET_DIR=/build/jbotci/target/python-wheels
export CARGO_HOME=/build/jbotci/scratch/python-wheels/cargo-home
export TMPDIR=/build/jbotci/scratch/python-wheels/tmp
mkdir -p "$CARGO_HOME" "$TMPDIR" /build/jbotci/scratch/python-wheels/dist
uv run --locked --project . --group dev maturin develop
uv run --locked --project . python tools/generate_syntax_models.py --check
uv run --locked --project . python tools/generate_domain_enum_stubs.py --check
uv run --locked --project . python tools/compose_stubs.py --check
uv run --locked --project . python tools/generate_api_matrix.py --check
```

On a host matching one matrix target, build the wheel twice with Maturin
1.14.1. Substitute the target triple from the table above; Linux builds must
run in Maturin's manylinux 2.28 container (the GitHub workflow is the exact
reference invocation):

```console
uv run --locked --project . --group dev maturin build \
  --release --locked --strip --compatibility pypi \
  --target aarch64-unknown-linux-gnu \
  -i python3.11 \
  --out /build/jbotci/scratch/python-wheels/dist/first
uv run --locked --project . --group dev maturin build \
  --release --locked --strip --compatibility pypi \
  --target aarch64-unknown-linux-gnu \
  -i python3.11 \
  --out /build/jbotci/scratch/python-wheels/dist/second
python tools/python_artifacts.py compare \
  --left /build/jbotci/scratch/python-wheels/dist/first/jbotci-*.whl \
  --right /build/jbotci/scratch/python-wheels/dist/second/jbotci-*.whl
python tools/python_artifacts.py inspect \
  --artifact /build/jbotci/scratch/python-wheels/dist/first/jbotci-*.whl \
  --kind wheel --platform linux-aarch64
python tools/run_wheel_tests.py \
  --wheel /build/jbotci/scratch/python-wheels/dist/first/jbotci-*.whl \
  --package-root "$PWD" \
  --workspace-root "$(git rev-parse --show-toplevel)" \
  --venv /build/jbotci/scratch/python-wheels/test-3.11
```

Invoke the last command once with CPython 3.11 and once with CPython 3.14.
For the sdist proof, build and inspect two sdists, unpack the first under
`/build`, and run the pinned Maturin container with **only the unpacked source
and a Cargo cache mounted**—never mount the checkout:

```console
uv run --locked --project . --group dev maturin sdist \
  --out /build/jbotci/scratch/python-wheels/sdist/raw-first
uv run --locked --project . --group dev maturin sdist \
  --out /build/jbotci/scratch/python-wheels/sdist/raw-second
mkdir -p /build/jbotci/scratch/python-wheels/sdist/{first,second}
raw_first=(/build/jbotci/scratch/python-wheels/sdist/raw-first/*.tar.gz)
raw_second=(/build/jbotci/scratch/python-wheels/sdist/raw-second/*.tar.gz)
python tools/python_artifacts.py normalize-sdist \
  --input "${raw_first[0]}" \
  --output "/build/jbotci/scratch/python-wheels/sdist/first/${raw_first[0]##*/}"
python tools/python_artifacts.py normalize-sdist \
  --input "${raw_second[0]}" \
  --output "/build/jbotci/scratch/python-wheels/sdist/second/${raw_second[0]##*/}"
python tools/python_artifacts.py compare \
  --left /build/jbotci/scratch/python-wheels/sdist/first/jbotci-*.tar.gz \
  --right /build/jbotci/scratch/python-wheels/sdist/second/jbotci-*.tar.gz
python tools/python_artifacts.py inspect \
  --artifact /build/jbotci/scratch/python-wheels/sdist/first/jbotci-*.tar.gz \
  --kind sdist
```

After unpacking, use the same `compare`, `inspect`, and `run_wheel_tests.py`
commands on the two sdist-built wheels. The workflow's offline build and
checkout-denial steps are the acceptance authority if local container tooling
is unavailable.
