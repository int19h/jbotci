# Cargo target cache

A Cargo unit is one target, profile, feature set, and compilation mode.
Each unit keeps a SHA-256 input record in its fingerprint directory.
When later jobs select different units, the record still describes the content that produced that unit.
CI uses Rust 1.100.0 and one target directory for each cached job.
The CLI release workflow never restores a target cache.

Both cached jobs export `JBOTCI_INPUT_SESSION` through `GITHUB_ENV` before their first Cargo step.
Every later step receives the session path under `RUNNER_TEMP`.
The job environment does not use the unavailable `runner` context.
Cargo and DX paths use the container's `RUNNER_TEMP`, outside the checkout.
The cache actions use those exported paths.
Copied native sources cannot discover the checkout as a parent Git repository.
When Render changes `HOME`, one `GIT_CONFIG_GLOBAL` file under `RUNNER_TEMP` keeps Git trust unchanged.
It trusts only the exact checkout and `vendor/cll` paths.
The trust step exercises recursive file listing and Git metadata reads before Cargo work.

The CLI release tooling job installs actionlint 1.7.12 and compares the archive with its pinned SHA-256 before extraction.
It runs actionlint on every workflow with ShellCheck enabled.
When actionlint is installed, the local Python workflow tests run the same command.
Install ShellCheck too before you run those tests.

Each cached job writes a time marker before its first Cargo call.
Cargo 1.99 and Cargo 1.100 leave the committed fingerprint mtime unchanged for fresh units.
Both versions write a new fingerprint mtime for rebuilt compile units, build-script binaries, and build-script runs.
Before a successful save, CI records each unit whose committed fingerprint is newer than that marker.
It preserves the records of fresh units and deletes units without valid records.
This also covers units built by nested Cargo commands and by dx.
It does not require Cargo JSON messages or a selected-unit set.

Compile records include every path in rustc's `.d` file and Cargo's implicit manifests and configuration.
Cargo 1.100 writes compile `.d` files under `build/<package>/<unit id>/out/`.
Cargo 1.99 writes build-script `.d` files under `build/<pkg>-<hash>`.
Cargo 1.99 writes other compile `.d` files under `deps` or `examples`.
Cargo's encoded dep-info omits immutable registry package sources, so it cannot replace the full rustc path list.
The record also preserves rustc's environment dependencies and Cargo's filtered environment dependencies.
Environment values use SHA-256 in the record, so the record does not expose secret values.
Cargo continues to compare environment values and compiler configuration independently of file mtimes.

Build-script run records include every `rerun-if-changed` input.
Directory inputs include recursive file content, membership, empty directories, and directory links.
A script without a file declaration uses its whole package directory, without Git metadata or target output.
Records also bind dependency records by SHA-256.
An unbuilt consumer cannot acquire a new dependency record after another unit rebuilds.
Dependency changes invalidate every affected variant, including variants that the current job does not select.
If several profiles share the same Cargo fingerprint, the record binds every match.
This conservative dependency set avoids a guess about the producing profile.

Each record has a format version and an integrity hash.
Workspace input paths are relative to the checkout.
Cargo home input paths remain absolute.
Owned target inputs use paths relative to the target directory.
A missing file, changed content, changed directory membership, or damaged record deletes the unit on restore.
The restore also deletes affected consumers.
Only then does it set tracked checkout files and directories to one fixed old time, including submodules.
It never invalidates changed content through a comparison between two runners' clocks.
There is no source commit stamp.

CI traces its existing builds rather than adding another cold build.
`strace -f --seccomp-bpf` follows build scripts, compiler processes, and nested Cargo or dx commands.
The kernel filter stops the tracer only for the requested system calls.
It avoids tracing every read, write, and memory mapping during compilation and linking.
The check compares repository file reads with each unit's declared input set.
It excludes test execution, immutable registry source, system files, and owned target output from this contract.
A deliberate undeclared read fails the regression fixture.
An unreadable input record for a workspace or vendored unit fails the contract check.
The trace parser retains file paths, unit metadata, and process identities.
It records the executable, parent PID, and argument count without argument values or environment values.
When the executable is inside the traced target, a rejected operation also records that path.
The trace decides a candidate from the operation position in the traced target.
When its name starts with `build-script-` or `build_script_`, an executable under a profile `build` directory is a build-script candidate.
When that executable is in the Cargo 1.100 unit place, the trace reads its fingerprint first.
When the fingerprint has an exact test or bench marker, the trace does not make the execution a unit.
The exact markers are `test-lib-`, `test-bin-`, `test-integration-test-`, `test-example-`, and `test-bench-`.
`test-bench-` is the bench marker.
The `rustc` compilation of that test remains a unit.
When the fingerprint is missing, the trace rejects the execution.
When the marker is another `test-` or `bench-` name, the trace rejects the execution.
A `.json` sidecar is not the marker.
An exact marker must be a regular file.
When the marker name is a directory, the trace rejects the execution.
When the marker name is another entry that is not a regular file, the trace rejects the execution.
A `rustc` call with Cargo metadata is a unit candidate.
The trace does not use environment values for that candidate decision.
When a candidate differs from a known Cargo layout, the trace rejects the operation.
A missing environment value, a malformed id, or an unknown shape is a rejection.
When an operation is outside those positions, the trace does not make it a unit.

The cache tool protects an honest build in the CI image.
It protects that build against configuration drift and against a layout change.
A traced build that deliberately places a fake `rustup` first on `PATH` is outside this tool.
A traced build that swaps the rustup home through a symlink during the trace is outside this tool.

The supported compiler is the real path of the pinned toolchain `rustc`.
Before the trace starts, the trace resolves that path from the traced workspace.
It runs `rustup which rustc` and then takes the real path.
The trace clears `RUSTUP_TOOLCHAIN` for that one resolution.
The workspace pin then selects the toolchain.
The trace reads the channel from `rust-toolchain.toml`.
The version line must equal `rustc` plus that channel, then a space or the end of the line.
When the resolved file is the rustup proxy, the trace rejects the build.
When the compiler version differs from that channel, the trace rejects the build.
When `rustup` is absent, the trace uses `rustc` on `PATH` and takes that real path.
When the exec name is `rustc` or `rustc.exe`, that exec is a compiler exec.
When the real path name is `rustc` or `rustc.exe`, that exec is a compiler exec.
When the exec carries `--crate-name` and `-C metadata=`, that exec is a compiler exec.
When the exec name has no slash, the trace resolves that name through `PATH`.
When the loaded file is the rustup proxy, the trace runs `rustup which rustc` in that process directory.
The selected file must be the supported compiler.
When a bare `rustc` name loads the rustup proxy, the resolved compiler file is the compile unit.
A call with Cargo metadata then records that unit.
When a compiler exec resolves to another file, the trace rejects the build.
The dx rules below name the exceptions.
The trace reports one error for that path.
The compiler configuration is not supported.
When a process sets `RUSTC`, that value must have this same real path.
When that value is a bare name or the rustup proxy, the trace resolves it the same way.
Cargo sets `RUSTC` on a build script after it resolves the compiler.
When a process sets `RUSTC_WRAPPER`, the trace rejects the build.
When `RUSTC_WRAPPER` is empty, the trace accepts the command.
When `RUSTC_WORKSPACE_WRAPPER` is empty, the trace accepts the command.

Before the trace starts, the trace sets `RUSTUP_AUTO_INSTALL` to `0`.
Rustup then does not download a toolchain during that trace.
The Render image workflow sets the same variable for every job.
`dx` is the Dioxus command.
Before the traced command starts, the trace looks up `dx` on `PATH`.
The trace reads the Dioxus CLI version from the Render image workflow.
The version line is `dioxus` plus that version.
One hex hash can follow inside parentheses after one space.
The hash has no spaces.
When the line has any other text, the trace rejects that file.
When the line matches, the trace records the real path of that file.
The trace accepts `RUSTC_WORKSPACE_WRAPPER` for this file only.
Any other workspace wrapper is rejected.
The compiler configuration is not supported.

Cargo runs that file as `dx`, then the compiler path, then the compiler arguments.
The compiler path is the second argument.
When the second argument does not resolve to the supported compiler, the trace rejects that exec.
The child `rustc` records the compile unit.
The wrapper exec does not record a unit.
The trace records that unit once.
When the rustup proxy then execs the supported compiler in that same process, the trace keeps the one record.

That `dx` also runs `cargo +nightly rustc --unit-graph` before the build.
When nightly is absent, that probe fails.
Then `dx` builds with the pinned compiler.
When a nightly `rustc` still runs under this `dx`, the trace accepts one probe form.
The probe has `-vV` or `--print`.
The probe has no Cargo metadata.
A nightly compile is rejected.
Cargo reads `.cargo/config.toml`, `.cargo/config`, and `config.toml` under `CARGO_HOME`.
When one of those files contains `build.rustc`, `build.rustc-wrapper`, or `build.rustc-workspace-wrapper`, the trace rejects the build.
The path rule rejects a compiler that those file checks do not name.
When Cargo passes `--target`, the build-script binary stays in the profile for the host.
`OUT_DIR` is under the `TARGET` value from that build script.
When `TARGET` is absent, the trace rejects that split layout.
When `TARGET` differs from that directory, the trace rejects the script.
The trace records the run unit next to `OUT_DIR`.
When the executable matches the Cargo 1.100 build-script place, a missing compile fingerprint is an error.
Strace's `-Y` option supplies process names, including names for short-lived threads.
For an unknown PID, the reader also attempts a live `/proc` snapshot for diagnostics.
Unavailable identity fields remain null. These snapshots never determine a Cargo unit's ownership.
Only reported fork results supply the parent context for input accounting.
An incomplete-process error prints these identities and any interrupted parent forks from the journal.

A process-wide exit can interrupt a clone before strace reports the new thread ID.
The new thread then emits only teardown placeholders and a termination event.
If every buffered event matches those exact forms, the parser closes that PID.
Its journal records the accepted placeholders and the explicit termination as `unattributed-exit`.
It never discards an exec, directory change, file open, or other unknown operation through this rule.
Those events remain buffered until the reported parent context arrives.

An abbreviated or genuinely incomplete trace still fails CI and prevents a cache save.
When the tests pass, an undeclared read must still fail the input contract.
Missing coverage cannot establish that contract, so CI does not replace this failure with a cache warning.
Each job also requires the checkout's tracked content to stay unchanged while it builds.

The native script declares the full C++ tree, patches, shim, wrapper, and build helper.
Each native script rerun replaces copied sources and native outputs.
Bindgen tracks original and system headers.
It does not register headers under this run's `OUT_DIR` as external inputs.
Those copied headers are outputs of the run, and their original tree and patches already supply the input records.
The UI and server declare Git inputs for their build tags in local builds.
Cached CI jobs supply `JBOTCI_GIT_COMMIT` for every Cargo call.
The scripts then use that value without reading Git files.
Per-job checkout credentials therefore do not invalidate otherwise unchanged units.
The dictionary and CLL scripts declare their source data and edition metadata.
The contract scanner declares the source trees that it scans.
`option_env!("JBOTCI_WEB_EMBEDDINGS_BASE_URL")` already appears in rustc's environment dependencies.
The regression proves that changing its value rebuilds the unit.

The `target-v5` keys reject every earlier cache design.
The key includes the full `rustc -vV`, runner OS, immutable container image digest, and installed system package hash.
An image resolution job pins the manifest before the cached job starts.
Every restore prefix also requires the same lockfile hash.
Only successful jobs save target caches.
DX state and other generated state stay out of the cache.
CI disables and removes incremental directories because their session hashes do not identify Cargo units.
Cargo recreates unhashed uplifted outputs from retained hashed artifacts after a restore.
