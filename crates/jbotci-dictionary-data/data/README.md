# Lensisku Dictionary Snapshot

This directory contains the vendored Lensisku cached dictionary exports owned
and compiled by `jbotci-dictionary-data`. Keeping the build inputs inside the
crate makes every Cargo source package—and therefore the Python sdist—complete
without reaching back into a repository checkout.

Refresh the English JSON snapshot with:

```sh
cargo xtask vendor-dictionary
```

Use `cargo xtask vendor-dictionary --check` in CI or review workflows to verify
that the current cached export still validates without rewriting the vendored
files.
