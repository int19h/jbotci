#!/usr/bin/env python3
"""Inspect and compare jbotci Python wheels and source distributions."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import re
import subprocess
import tarfile
import tempfile
import tomllib
import zipfile
from dataclasses import asdict, dataclass
from pathlib import Path, PurePosixPath

PACKAGE_ROOT = Path(__file__).resolve().parents[1]
POLICY_PATH = PACKAGE_ROOT / "artifact-policy.toml"

REQUIRED_WHEEL_FILES: frozenset[str] = frozenset(
    {
        "jbotci/__init__.py",
        "jbotci/_errors.py",
        "jbotci/_native.pyi",
        "jbotci/_validation.py",
        "jbotci/diagnostics.py",
        "jbotci/dialect.py",
        "jbotci/dictionary.py",
        "jbotci/jvozba.py",
        "jbotci/morphology.py",
        "jbotci/py.typed",
        "jbotci/semantics/__init__.py",
        "jbotci/semantics/references.py",
        "jbotci/source.py",
        "jbotci/syntax/__init__.py",
        "jbotci/syntax/_runtime.py",
        "jbotci/syntax/recovered.py",
        "jbotci/syntax/recovered.pyi",
        "jbotci/syntax/strict.py",
        "jbotci/syntax/strict.pyi",
    }
)

REQUIRED_SDIST_FILES: frozenset[str] = frozenset(
    {
        "Cargo.lock",
        "Cargo.toml",
        "LICENSE.md",
        "PKG-INFO",
        "README.md",
        "pyproject.toml",
        "bindings/python/Cargo.toml",
        "bindings/python/LICENSE.md",
        "bindings/python/README.md",
        "bindings/python/artifact-policy.toml",
        "bindings/python/build.rs",
        "bindings/python/examples/dictionary_jvozba.py",
        "bindings/python/examples/end_to_end.py",
        "bindings/python/examples/recovery_diagnostics.py",
        "bindings/python/stubs/_native/diagnostics.pyi",
        "bindings/python/stubs/_native/dialect.pyi",
        "bindings/python/stubs/_native/dictionary.pyi",
        "bindings/python/stubs/_native/domain_enums.pyi",
        "bindings/python/stubs/_native/generated.pyi",
        "bindings/python/stubs/_native/jvozba.pyi",
        "bindings/python/stubs/_native/manual.pyi",
        "bindings/python/stubs/_native/morphology.pyi",
        "bindings/python/stubs/_native/parser.pyi",
        "bindings/python/stubs/_native/references.pyi",
        "bindings/python/stubs/_native/source.pyi",
        "bindings/python/stubs/_native/syntax.pyi",
        "bindings/python/tests/typecheck.py",
        "bindings/python/tools/compose_stubs.py",
        "bindings/python/tools/generate_domain_enum_stubs.py",
        "bindings/python/tools/generate_syntax_models.py",
        "bindings/python/tools/installed_package.py",
        "bindings/python/tools/python_artifacts.py",
        "bindings/python/tools/run_installed_examples.py",
        "bindings/python/tools/run_wheel_tests.py",
        "bindings/python/uv.lock",
        "crates/bityzba/LICENSE.md",
        "crates/bityzba/README.md",
        "crates/bityzba-contract-syntax/LICENSE.md",
        "crates/bityzba-macros/LICENSE.md",
        "crates/jbotci-dictionary-data/data/dictionary-en.json",
        "crates/jbotci-dictionary-data/data/dictionary-en.metadata.toml",
        "crates/jbotci-syntax/src/grammar/generated.rs",
        "crates/jbotci-syntax-macros/src/lib.rs",
        "python/jbotci/_native.pyi",
        "python/jbotci/py.typed",
        "python/jbotci/syntax/recovered.pyi",
        "python/jbotci/syntax/strict.pyi",
    }
)

REQUIRED_SDIST_CRATES: tuple[str, ...] = (
    "bindings/python",
    "bindings/python/syntax-macros",
    "crates/bityzba",
    "crates/bityzba-contract-syntax",
    "crates/bityzba-macros",
    "crates/jbotci-diagnostics",
    "crates/jbotci-dialect",
    "crates/jbotci-dictionary",
    "crates/jbotci-dictionary-data",
    "crates/jbotci-jvozba",
    "crates/jbotci-morphology",
    "crates/jbotci-phonetic",
    "crates/jbotci-semantics",
    "crates/jbotci-source",
    "crates/jbotci-syntax",
    "crates/jbotci-syntax-macros",
    "crates/jbotci-tree",
    "crates/jbotci-tree-macros",
)

FORBIDDEN_COMPONENTS: frozenset[str] = frozenset(
    {
        ".git",
        ".github",
        ".github-cache",
        ".mypy_cache",
        ".pytest_cache",
        ".venv",
        "__pycache__",
        "node_modules",
        "target",
    }
)

SECRET_FILE_RE = re.compile(
    r"(^|/)(?:\.env(?:\..*)?|id_(?:rsa|dsa|ecdsa|ed25519)|"
    r"[^/]*\.(?:key|pem|p12|pfx))$",
    re.IGNORECASE,
)
GENERIC_LOCAL_PATHS: tuple[bytes, ...] = (
    b"/home/" + b"runner/work/",
    b"/Users/" + b"runner/work/",
    b"D:" + b"\\a\\",
)
WHEEL_PLATFORM_MARKERS: dict[str, tuple[str, ...]] = {
    "linux-x86_64": ("manylinux_2_28", "x86_64.whl"),
    "linux-aarch64": ("manylinux_2_28", "aarch64.whl"),
    "macos-x86_64": ("macosx_", "x86_64.whl"),
    "macos-aarch64": ("macosx_", "arm64.whl"),
    "windows-x86_64": ("win_amd64.whl",),
    "sdist-wheel-x86_64": ("manylinux_", "x86_64.whl"),
    "sdist-wheel-aarch64": ("manylinux_", "aarch64.whl"),
}

# Maturin trims the packaged workspace members to the binding dependency
# closure but retains unrelated root workspace path dependencies. Keep this
# exact audited list rather than shipping their large source trees. Every
# remaining path dependency is checked against a packaged Cargo.toml below.
SDIST_OMITTED_WORKSPACE_DEPENDENCIES: tuple[bytes, ...] = (
    b'xtask-common = { path = "xtask-common" }\n',
    b'llama-cpp-sys-4 = { path = "crates/vendor/llama-cpp-sys-4", default-features = false }\n',
    b'jbotci-cll = { path = "crates/jbotci-cll" }\n',
    b'jbotci-embedding-inputs = { path = "crates/jbotci-embedding-inputs" }\n',
    b'jbotci-embeddings = { path = "crates/jbotci-embeddings" }\n',
    b'jbotci-f2llm-runtime = { path = "crates/jbotci-f2llm-runtime" }\n',
    b'jbotci-gentufa = { path = "crates/jbotci-gentufa" }\n',
    b'jbotci-gimfihi = { path = "crates/jbotci-gimfihi" }\n',
    b'jbotci-ide = { path = "crates/jbotci-ide" }\n',
    b'jbotci-orthography = { path = "crates/jbotci-orthography" }\n',
    b'jbotci-output = { path = "crates/jbotci-output" }\n',
    b'jbotci-search = { path = "crates/jbotci-search" }\n',
    b'jbotci-ui = { path = "crates/jbotci-ui" }\n',
    b'jbotci-web-core = { path = "crates/jbotci-web-core" }\n',
)
SDIST_OMITTED_PATCH = (
    b"[patch.crates-io]\n"
    b'llama-cpp-sys-4 = { path = "crates/vendor/llama-cpp-sys-4" }\n\n'
)


@dataclass(frozen=True)
class ArtifactReceipt:
    """Stable inspection output uploaded alongside each workflow artifact."""

    artifact: str
    kind: str
    platform: str
    archive_bytes: int
    unpacked_bytes: int
    entries: int
    sha256: str


def parse_args() -> argparse.Namespace:
    """Parse artifact-tool arguments."""
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    inspect_parser = subparsers.add_parser("inspect")
    inspect_parser.add_argument("--artifact", required=True, type=Path)
    inspect_parser.add_argument(
        "--kind", required=True, choices=("wheel", "sdist")
    )
    inspect_parser.add_argument("--platform", default="")
    inspect_parser.add_argument("--forbid-path", action="append", default=[])
    inspect_parser.add_argument("--receipt", type=Path)

    normalize_parser = subparsers.add_parser("normalize-sdist")
    normalize_parser.add_argument("--input", required=True, type=Path)
    normalize_parser.add_argument("--output", required=True, type=Path)

    compare_parser = subparsers.add_parser("compare")
    compare_parser.add_argument("--left", required=True, type=Path)
    compare_parser.add_argument("--right", required=True, type=Path)

    table_parser = subparsers.add_parser("size-table")
    table_parser.add_argument("receipts", nargs="+", type=Path)
    table_parser.add_argument("--output", type=Path)
    return parser.parse_args()


def _safe_name(name: str) -> PurePosixPath:
    path = PurePosixPath(name)
    assert not path.is_absolute(), name
    assert ".." not in path.parts, name
    assert "\\" not in name, name
    return path


def _wheel_entries(path: Path) -> dict[str, bytes]:
    entries: dict[str, bytes] = {}
    with zipfile.ZipFile(path) as archive:
        for info in archive.infolist():
            name = str(_safe_name(info.filename))
            if info.is_dir():
                continue
            unix_mode = info.external_attr >> 16
            assert unix_mode & 0o170000 != 0o120000, f"symlink in wheel: {name}"
            assert name not in entries, f"duplicate wheel member: {name}"
            entries[name] = archive.read(info)
    return entries


def _sdist_entries(path: Path) -> dict[str, bytes]:
    raw_entries: dict[str, bytes] = {}
    roots: set[str] = set()
    with tarfile.open(path, "r:*") as archive:
        for member in archive.getmembers():
            safe = _safe_name(member.name)
            assert safe.parts, member.name
            roots.add(safe.parts[0])
            if member.isdir():
                continue
            assert member.isfile(), f"non-regular sdist member: {member.name}"
            relative = PurePosixPath(*safe.parts[1:])
            assert relative.parts, member.name
            name = str(relative)
            assert name not in raw_entries, f"duplicate sdist member: {name}"
            stream = archive.extractfile(member)
            assert stream is not None
            raw_entries[name] = stream.read()
    assert len(roots) == 1, f"sdist has multiple roots: {sorted(roots)}"
    return raw_entries


def archive_entries(path: Path) -> dict[str, bytes]:
    """Return logical archive members without archive metadata."""
    if path.name.endswith(".whl"):
        return _wheel_entries(path)
    if path.name.endswith((".tar.gz", ".tar.bz2", ".tar.xz")):
        return _sdist_entries(path)
    raise AssertionError(f"unsupported artifact extension: {path}")


def _check_limits(path: Path, entries: dict[str, bytes], key: str) -> None:
    """Assert the per-file upload tripwire and the archive's entry count.

    Artifact size otherwise carries no ceiling here; the receipt this
    inspection returns is what records it. See `artifact-policy.toml`.
    """
    policy = tomllib.loads(POLICY_PATH.read_text(encoding="utf-8"))
    max_artifact = policy["limits"]["max_artifact_bytes"]
    archive_size = path.stat().st_size
    assert archive_size <= max_artifact, (archive_size, max_artifact, key)
    max_entries = policy["entries"][key]
    assert len(entries) <= max_entries, (len(entries), max_entries, key)


def _check_common_content(
    entries: dict[str, bytes], forbidden_paths: tuple[bytes, ...]
) -> None:
    for name, contents in entries.items():
        path = PurePosixPath(name)
        assert not (set(path.parts) & FORBIDDEN_COMPONENTS), name
        assert SECRET_FILE_RE.search(name) is None, name
        for forbidden in forbidden_paths:
            assert forbidden not in contents, f"local path leaked in {name}"
        assert not any(
            local_path in contents for local_path in GENERIC_LOCAL_PATHS
        ), f"generic CI-local path leaked in {name}"
        if path.suffix.lower() in {
            ".json",
            ".md",
            ".py",
            ".pyi",
            ".rs",
            ".toml",
            ".tsv",
            ".txt",
        }:
            assert b"file:" + b"//" not in contents, (
                f"local file URL leaked in {name}"
            )


def _check_wheel(entries: dict[str, bytes], platform: str) -> None:
    assert platform, "wheel inspection requires --platform"
    assert platform in WHEEL_PLATFORM_MARKERS, platform
    assert REQUIRED_WHEEL_FILES <= entries.keys()
    native = [
        name
        for name in entries
        if name.startswith("jbotci/_native.")
        and name.endswith((".so", ".pyd", ".dylib"))
    ]
    assert len(native) == 1, native
    dist_info = {
        name.split("/", 1)[0]
        for name in entries
        if name.endswith(".dist-info/METADATA")
    }
    assert len(dist_info) == 1, dist_info
    dist_info_dir = next(iter(dist_info))
    for relative in ("METADATA", "WHEEL", "RECORD", "licenses/LICENSE.md"):
        assert f"{dist_info_dir}/{relative}" in entries
    assert not any("/sboms/" in name for name in entries)
    assert all(
        name.startswith("jbotci/") or name.startswith(f"{dist_info_dir}/")
        for name in entries
    )
    for name, contents in entries.items():
        if len(contents) <= 2_000_000:
            continue
        # A SHAPE check, deliberately without a byte ceiling: the extension
        # module is the only member a wheel may carry above 2 MB, so anything
        # else this large is an accident -- a vendored data file, a stray build
        # artefact, a second binary -- rather than the packaged code growing.
        #
        # This assertion used to carry `<= 115_000_000` on the member as well.
        # That bound was a survivor of the per-platform byte budgets the owner
        # retired on 2026-08-19 (see `artifact-policy.toml`): it was
        # project-invented, no publishing target imposes anything on a member
        # INSIDE the archive, and its own comment prescribed the very
        # recalibrate-every-epoch cycle the ruling ended -- epochs 3 through 6b
        # each spent a commit on it, and epoch 8 was the next. The real
        # constraint is PyPI's 100 MiB per-file limit on the DISTRIBUTED FILE,
        # and `_check_limits` still asserts it at a 95 MiB tripwire; the
        # largest wheel this project builds is 24.1 MB, 23% of that limit,
        # because the member is compressed roughly 4.5:1 on every platform.
        assert name == native[0], (name, len(contents))


def _check_sdist(entries: dict[str, bytes]) -> None:
    assert REQUIRED_SDIST_FILES <= entries.keys(), sorted(
        REQUIRED_SDIST_FILES - entries.keys()
    )
    for crate in REQUIRED_SDIST_CRATES:
        assert f"{crate}/Cargo.toml" in entries, crate
        assert any(name.startswith(f"{crate}/src/") for name in entries), crate
    manifest = tomllib.loads(entries["Cargo.toml"].decode("utf-8"))
    for dependency in manifest["workspace"]["dependencies"].values():
        if isinstance(dependency, dict) and "path" in dependency:
            assert f"{dependency['path']}/Cargo.toml" in entries, dependency
    for patch_source in manifest.get("patch", {}).values():
        for dependency in patch_source.values():
            assert f"{dependency['path']}/Cargo.toml" in entries, dependency
    for name, contents in entries.items():
        if name.startswith("crates/") and "/tests/" in name:
            assert name.startswith(
                "crates/jbotci-syntax-macros/tests/"
                "binding-schema-consumer/"
            ), name
        assert "/ui/" not in name, name
        assert "/benches/" not in name, name
        assert "/phaseb_corpus/" not in name, name
        assert name != "bindings/python/docs/api-parity.tsv", name
        if len(contents) <= 1_500_000:
            continue
        allowed_large_members = {
            "crates/jbotci-dictionary-data/data/dictionary-en.json": 10_000_000,
        }
        assert (
            name in allowed_large_members
            and len(contents) <= allowed_large_members[name]
        ), (name, len(contents))


def _normalized_workspace_manifest(contents: bytes) -> bytes:
    """Remove exact unrelated path references retained by Maturin."""
    for dependency in SDIST_OMITTED_WORKSPACE_DEPENDENCIES:
        assert contents.count(dependency) == 1, dependency
        contents = contents.replace(dependency, b"")
    assert contents.count(SDIST_OMITTED_PATCH) == 1
    return contents.replace(SDIST_OMITTED_PATCH, b"")


def normalize_sdist(input_path: Path, output_path: Path) -> None:
    """Write a deterministic standalone sdist from Maturin's raw archive."""
    input_path = input_path.resolve()
    output_path = output_path.resolve()
    assert input_path.is_file(), input_path
    assert input_path != output_path
    assert input_path.name.endswith(".tar.gz"), input_path
    assert output_path.name.endswith(".tar.gz"), output_path
    output_path.parent.mkdir(parents=True, exist_ok=True)

    with tempfile.TemporaryDirectory(
        prefix="jbotci-normalize-sdist-",
        dir=output_path.parent,
    ) as directory:
        extraction_root = Path(directory)
        members: list[tuple[str, bytes, int]] = []
        root_manifest: Path | None = None
        with tarfile.open(input_path, "r:gz") as source:
            for member in source.getmembers():
                safe = _safe_name(member.name)
                assert member.isdir() or member.isfile(), member.name
                target = extraction_root.joinpath(*safe.parts)
                if member.isdir():
                    target.mkdir(parents=True, exist_ok=True)
                    members.append((member.name, tarfile.DIRTYPE, member.mode))
                    continue
                target.parent.mkdir(parents=True, exist_ok=True)
                stream = source.extractfile(member)
                assert stream is not None
                contents = stream.read()
                if len(safe.parts) == 2 and safe.parts[1] == "Cargo.toml":
                    assert root_manifest is None
                    root_manifest = target
                    contents = _normalized_workspace_manifest(contents)
                target.write_bytes(contents)
                target.chmod(member.mode)
                members.append((member.name, tarfile.REGTYPE, member.mode))

        assert root_manifest is not None
        # Resolve the workspace that is actually packaged, and let cargo write
        # the lockfile that describes it. A source distribution is a pruned
        # workspace: Maturin ships this repository's own lockfile but cuts the
        # workspace down to the binding dependency closure, and a lockfile for
        # the full repository workspace is not a valid lockfile for the smaller
        # one. Downstream consumers, including the isolated round trip's
        # `cargo fetch --locked` and `maturin build --locked`, refuse to update
        # a lockfile, so the archive has to leave here already carrying one that
        # matches its own workspace. This unlocked run is what produces it.
        subprocess.run(
            [
                "cargo",
                "metadata",
                "--manifest-path",
                str(root_manifest),
                "--format-version",
                "1",
            ],
            stdout=subprocess.DEVNULL,
            check=True,
        )
        # Then fail closed unless the rewritten workspace still loads from the
        # archive alone. `--no-deps` resolves no dependency graph, so this
        # proves the root manifest and every packaged member manifest parse and
        # resolve offline after the edits above; it does not re-check the
        # lockfile, whose agreement the run above has just established and the
        # `--locked` consumers enforce again.
        subprocess.run(
            [
                "cargo",
                "metadata",
                "--manifest-path",
                str(root_manifest),
                "--format-version",
                "1",
                "--locked",
                "--offline",
                "--no-deps",
            ],
            stdout=subprocess.DEVNULL,
            check=True,
        )

        with (
            output_path.open("wb") as raw_output,
            gzip.GzipFile(
                filename="",
                mode="wb",
                compresslevel=9,
                fileobj=raw_output,
                mtime=0,
            ) as compressed_output,
            tarfile.open(
                fileobj=compressed_output,
                mode="w",
                format=tarfile.PAX_FORMAT,
            ) as destination,
        ):
            for name, member_type, mode in members:
                safe = _safe_name(name)
                source_path = extraction_root.joinpath(*safe.parts)
                contents = (
                    source_path.read_bytes()
                    if member_type == tarfile.REGTYPE
                    else b""
                )
                normalized = tarfile.TarInfo(name)
                normalized.type = member_type
                normalized.mode = mode
                normalized.size = len(contents)
                normalized.mtime = 0
                normalized.uid = 0
                normalized.gid = 0
                normalized.uname = ""
                normalized.gname = ""
                destination.addfile(
                    normalized,
                    io.BytesIO(contents)
                    if member_type == tarfile.REGTYPE
                    else None,
                )
    print(f"normalized standalone sdist: {output_path}")


def inspect_artifact(args: argparse.Namespace) -> ArtifactReceipt:
    """Inspect content and limits, returning a machine-readable receipt."""
    path = args.artifact.resolve()
    assert path.is_file(), path
    entries = archive_entries(path)
    forbidden_paths = tuple(
        value.encode("utf-8")
        for value in {
            str(PACKAGE_ROOT.parent.parent.resolve()),
            *(str(Path(value).resolve()) for value in args.forbid_path),
        }
    )
    _check_common_content(entries, forbidden_paths)
    if args.kind == "wheel":
        assert path.name.endswith(".whl"), path
        assert "-cp311-abi3-" in path.name, path.name
        assert args.platform in WHEEL_PLATFORM_MARKERS, args.platform
        assert all(
            marker in path.name
            for marker in WHEEL_PLATFORM_MARKERS[args.platform]
        ), (path.name, args.platform)
        _check_wheel(entries, args.platform)
        policy_key = args.platform
    else:
        assert path.name.endswith(".tar.gz"), path
        _check_sdist(entries)
        policy_key = "sdist"
    _check_limits(path, entries, policy_key)
    receipt = ArtifactReceipt(
        artifact=path.name,
        kind=args.kind,
        platform=args.platform or "source",
        archive_bytes=path.stat().st_size,
        unpacked_bytes=sum(len(value) for value in entries.values()),
        entries=len(entries),
        sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
    )
    if args.receipt is not None:
        args.receipt.parent.mkdir(parents=True, exist_ok=True)
        args.receipt.write_text(
            json.dumps(asdict(receipt), indent=2) + "\n",
            encoding="utf-8",
        )
    print(json.dumps(asdict(receipt), sort_keys=True))
    return receipt


def compare_artifacts(left: Path, right: Path) -> None:
    """Require logical members to match while ignoring archive metadata."""
    left_entries = archive_entries(left.resolve())
    right_entries = archive_entries(right.resolve())
    assert left_entries.keys() == right_entries.keys(), (
        sorted(left_entries.keys() - right_entries.keys()),
        sorted(right_entries.keys() - left_entries.keys()),
    )
    changed = [
        name
        for name in left_entries
        if left_entries[name] != right_entries[name]
    ]
    assert not changed, f"logical archive members differ: {changed}"
    print(
        f"{left.name} and {right.name}: "
        f"{len(left_entries)} logical members are byte-identical"
    )


def size_table(receipts: list[Path], output: Path | None) -> None:
    """Render receipt JSON files as a Markdown artifact-size table."""
    values = [
        json.loads(path.read_text(encoding="utf-8"))
        for path in receipts
    ]
    values.sort(key=lambda value: (value["kind"], value["platform"]))
    lines = [
        "| Artifact | Platform | Compressed bytes | Unpacked bytes | Entries |",
        "| --- | --- | ---: | ---: | ---: |",
    ]
    lines.extend(
        "| {artifact} | {platform} | {archive_bytes} | "
        "{unpacked_bytes} | {entries} |".format(**value)
        for value in values
    )
    text = "\n".join(lines) + "\n"
    if output is not None:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(text, encoding="utf-8")
    print(text, end="")


def main() -> int:
    """Dispatch one artifact operation."""
    args = parse_args()
    if args.command == "inspect":
        inspect_artifact(args)
    elif args.command == "normalize-sdist":
        normalize_sdist(args.input, args.output)
    elif args.command == "compare":
        compare_artifacts(args.left, args.right)
    else:
        size_table(args.receipts, args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
