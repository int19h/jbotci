# Cargo target cache

A Cargo unit is one target, profile, feature set, and compilation mode.
Each cached job collects the units that Cargo selects, including fresh units.
The job removes every other unit before it stamps and saves the target directory.
The stamp identifies the source commit of all retained units.
The restore action backdates unchanged files against that commit and touches changed files.

`test.yml` collects workspace tests, fixture commands, and nested Cargo commands.
`render-image.yml` also collects xtask and dx commands for host and wasm builds.
The CLI release workflow uses fresh runners and does not restore a target cache.
Cargo home and the installed Dioxus CLI use separate caches.
The `target-v4` prefixes reject every earlier target cache.

`tools/cargo_cache.py install` creates a private Cargo proxy in the runner's temporary directory.
It copies stable Cargo as `cargo-real` and sets both `PATH` and `CARGO` to the proxy.
The proxy also collects the Dioxus CLI installation.
Its declared target directory is separate from the saved target cache.
The collector reports those units but does not retain or prune that uncached directory.
The alternate executable name lets Cargo preserve that `CARGO` value for nested commands.
A regression exercises this behavior with the installed stable Cargo.
The Rust toolchain and Rust language remain stable.

The proxy requests `--message-format=json` for every compilation command.
It records `compiler-artifact` messages and maps files to Cargo's unit directories.
It records build-script compile units from their executable paths.
It records build-script run units from `build-script-executed.out_dir`.
The [Cargo documentation](https://doc.rust-lang.org/cargo/reference/external-tools.html#build-script-output) states that Cargo emits this message for fresh runs too.
A regression requires both fresh compile messages and the cached build-script output.
The proxy preserves JSON for callers such as dx and renders diagnostics for other callers.
It passes program output through after `build-finished`.

A compiler guard records any Cargo process that bypasses the proxy.
An incomplete invocation, unsupported command, unsupported artifact layout, or unreported compiler rejects the save.
Expected compile failures use separate target directories, which the collector discards.
A failed invocation for the main target directory rejects the save.
The collector retains the union of selected units from all successful invocations.
It removes unselected fingerprints, dependency files, build directories, and root aliases.
CI disables and removes incremental directories because their session hashes differ from Cargo unit hashes.

The native build watches its original C++ tree, patches, shim, helper, wrapper, and prebuilt input directory.
Each script rerun replaces its copied source and native outputs.
It no longer uses a timestamp sentinel or a shared CMake cache outside Cargo's unit directories.
Contract scripts watch the source directories that they scan.
The dictionary script declares its dictionary, metadata, and extracted rafsi inputs.
The CLL script declares its chapter directory and edition metadata.
The UI and server scripts declare Git inputs and the explicit commit environment variable.
Cargo tracks Rust module and `include_str!` or `include_bytes!` inputs through compiler dependency information.
