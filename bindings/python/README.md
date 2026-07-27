# jbotci Python bindings

`jbotci` is a pre-alpha, typed Python interface to the jbotci Rust libraries.
The API is intentionally unstable: until a stable release policy is announced,
minor releases and development snapshots may rename symbols, change generated
syntax unions, or alter signatures without a deprecation period. Pin the exact
package version and run strict type checking when upgrading.

The public API starts at `jbotci` and its namespace modules. Import names from
those modules only; implementation modules are not part of the compatibility
surface.

## Install

CPython 3.11 or newer is required. To build and install a wheel from a checkout:

```console
cd bindings/python
maturin build --release
python -m venv .venv
.venv/bin/python -m pip install ../../target/wheels/jbotci-*.whl
```

For editable development:

```console
cd bindings/python
uv sync --group dev
uv run --group dev maturin develop
uv run --group dev python -m pytest -q tests
uv run --group dev mypy --strict python tests
```

`maturin develop` installs the mixed Rust/Python package into the selected
environment. Merely placing the checkout on `PYTHONPATH` is not sufficient.

## Start here

[`examples/end_to_end.py`](examples/end_to_end.py) is the complete typed
pipeline: parse source, structurally match generated nodes, inspect place and
reference analysis, and look up every constituent word in the embedded
dictionary. It does not serialize trees or use implementation-only modules.

Two smaller installed-package programs cover
[`dictionary_jvozba.py`](examples/dictionary_jvozba.py) and
[`recovery_diagnostics.py`](examples/recovery_diagnostics.py). Pytest discovers
and executes every program in `examples/` outside the source directory, and CI
also executes the same programs from a freshly installed wheel.

The [Python bindings guide](docs/guide.md) explains namespaces,
representations, strict and recovered parsing, traversal, dictionary and
jvozba use, references, diagnostics, ownership, and current non-goals. The
[Rust API parity matrix](docs/api-parity.md) documents exactly how every
in-scope public Rust item is represented or intentionally excluded.
[Python distribution artifacts](docs/artifacts.md) documents the wheel matrix,
clean installed-package tests, isolated sdist round trip, size ratchet,
reproducibility comparison, trigger policy, and owner reproduction commands.

Contributors extending the package should also read the
[binding conventions](docs/binding-conventions.md), including the pyclass,
ownership, collection, error, enum, registration, and native-export rules.
