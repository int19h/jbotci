# Lensisku Dictionary Snapshot

This directory contains vendored Lensisku cached dictionary exports used to
generate `jbotci-dictionary-data`.

Refresh the English JSON snapshot with:

```sh
cargo xtask vendor-dictionary
```

Use `cargo xtask vendor-dictionary --check` in CI or review workflows to verify
that the current cached export still validates without rewriting the vendored
files.
