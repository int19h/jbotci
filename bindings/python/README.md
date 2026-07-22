# jbotci Python bindings

This directory contains the pre-alpha `jbotci` Python distribution. Its public
API tracks the unstable Rust API and can change without compatibility notice.
The extension is private (`jbotci._native`); callers should import public names
from `jbotci` and its namespace modules.

## Local development

From this directory, create and populate a CPython 3.11-or-newer environment:

```sh
uv venv --python 3.11
uv sync --group dev
source .venv/bin/activate
```

Build and install the editable mixed package, then run its checks:

```sh
CARGO_TARGET_DIR=/var/tmp/jbotci-target uv run --group dev maturin develop
CARGO_TARGET_DIR=/var/tmp/jbotci-target cargo test -r -p jbotci-python
uv run --group dev python -m pytest -q tests
uv run --group dev mypy --strict python tests
```

Use a target directory on a local filesystem when the repository itself is on
a mounted filesystem. `maturin develop` installs both the Python sources and
the ABI3 native extension into the active environment. Imports should also be
smoke-tested with a working directory outside this repository so the editable
install, rather than the source tree, resolves `jbotci`.

## Binding conventions

- Public Python objects use explicit Python names and `module = "jbotci..."`.
  Pyclasses are frozen and non-subclassable by default. Value objects implement
  equality, and implement hashing only when the underlying Rust value is also
  hashable. Their `repr` is constructor-shaped and includes the public module.
- `Option<T>` maps to `T | None`. Rust sequences are materialized as immutable
  Python tuples, never mutable lists. Numeric newtypes keep their semantic
  Python class and validate range/units in the constructor instead of leaking a
  bare integer or a lossy cast.
- A Python wrapper for borrowed Rust data must retain a strong owner. The
  internal `OwnedReference` helper stores an `Arc` plus a projection function;
  wrappers must never expose Rust lifetimes or construct self-references. Static
  embedded data may use a `'static` reference directly.
- Structured Rust binding errors convert in one place to subclasses of
  `JbotciError`. Python-visible exceptions use stable messages and arguments;
  Rust implementation types (`Box`, `Arc`, bityzba data wrappers, lifetimes,
  and serde payloads) never cross the boundary.
- Fieldless Rust enums that represent finite choices become genuine
  `enum.Enum` classes, preferably with stable canonical string values. Payload
  enums and grammar ADTs instead become a closed set of frozen,
  non-subclassable variant/value classes with a typed union or base where
  appropriate. Neither form exposes Rust discriminants, declaration order,
  serde tags, or generated AST layout. The Rust grammar/schema generator is the
  only source of members and variants—Python never maintains a second list—and
  every generated value class is installed through the shared registration
  helper. The `PythonStringEnum` metadata and registration/conversion helpers
  are the reusable path for fieldless enums.
- Registration runs in a fixed order during module initialization. There is no
  mutable module-level Rust state; registrations and embedded data must be
  immutable or interpreter-local.

The checked-in `python/jbotci/_native.pyi` is composed from the ordered manual
and generated fragments under `stubs/_native/`. After changing either fragment,
run:

```sh
uv run python tools/compose_stubs.py
uv run python tools/compose_stubs.py --check
```

The explicit fragment manifest in the compositor is the ordering authority.
The test suite verifies that composition is reproducible and that the composed
public declarations match the native module's runtime exports.
