# Cargo target cache

A Cargo unit is one target, profile, feature set, and compilation mode.
Each unit keeps a SHA-256 input record in its fingerprint directory.
The record describes the content that produced that unit, even when later jobs select different units.
CI uses stable Rust and one target directory for each cached job.
The CLI release workflow never restores a target cache.

Each cached job writes a time marker before its first Cargo call.
Cargo 1.99 leaves the committed fingerprint mtime unchanged for fresh units.
It writes a new fingerprint mtime for rebuilt compile units, build-script binaries, and build-script runs.
Before a successful save, CI records each unit whose committed fingerprint is newer than that marker.
It preserves the records of fresh units and deletes units without valid records.
This also covers units built by nested Cargo commands and by dx.
It does not require Cargo JSON messages or a selected-unit set.

Compile records include every path in rustc's `.d` file and Cargo's implicit manifests and configuration.
Build-script binaries use their `.d` files under `build/<pkg>-<hash>`.
Other compile units use their `.d` files under `deps` or `examples`.
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
`strace -f` follows build scripts, compiler processes, and nested Cargo or dx commands.
The check compares repository file reads with each unit's declared input set.
It excludes test execution, immutable registry source, system files, and owned target output from this contract.
A deliberate undeclared read fails the regression fixture.
The trace parser retains only file paths and unit metadata.
It never stores raw command arguments or the full environment.
An abbreviated or incomplete trace rejects the check.
Each job also requires the checkout's tracked content to stay unchanged while it builds.

The native script declares the full C++ tree, patches, shim, wrapper, and build helper.
Each native script rerun replaces copied sources and native outputs.
The UI and server declare Git inputs for their build tags.
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
