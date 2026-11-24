"""Bind each cached Cargo unit to the content of its declared inputs."""
from __future__ import annotations

import argparse
from dataclasses import dataclass
from enum import Enum
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import tomllib
from typing import Any
from uuid import uuid4

from tools.cargo_inputs_trace import TraceError, trace
from tools.cargo_unit_layout import (
    RECORD, UNIT_ID, empty_fingerprint_hash, inside_unit_out, profile_inside_target,
)

VERSION = 1
OLD_TIME_NS = 946684800_000000000
HASH = re.compile(r"[0-9a-f]{64}")
UNIT_HASH = re.compile(r"-([0-9a-f]{16})(?:\.|$)")


class InputError(Exception):
    """The input record cannot establish that an artifact is current."""


class Kind(str, Enum):
    COMPILE = "compile"
    RUN = "run"


def canonical(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode()


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def atomic_json(path: Path, value: Any) -> None:
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_bytes(canonical(value) + b"\n")
    temporary.replace(path)


def remove(path: Path) -> None:
    if path.is_dir() and not path.is_symlink():
        shutil.rmtree(path)
    else:
        path.unlink(missing_ok=True)


@dataclass(frozen=True)
class Layout:
    workspace: Path
    target: Path
    profiles: tuple[Path, ...] | None = None

    def __post_init__(self) -> None:
        if not self.workspace.is_absolute() or not self.target.is_absolute():
            raise InputError("Workspace and target paths must be absolute")
        if self.workspace.is_relative_to(self.target):
            raise InputError("The target directory must not contain the checkout")
        if self.profiles and any(not path.is_relative_to(self.target) or path == self.target for path in self.profiles):
            raise InputError("Each selected profile must stay inside the target directory")

    def path_key(self, path: Path) -> str:
        path = Path(os.path.abspath(path))
        cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo")).absolute()
        if path.is_relative_to(cargo_home):
            return "absolute:" + str(path)
        if path.is_relative_to(self.target):
            return "target:" + path.relative_to(self.target).as_posix()
        if path.is_relative_to(self.workspace):
            return "workspace:" + path.relative_to(self.workspace).as_posix()
        return "absolute:" + str(path)

    def path(self, key: str) -> Path:
        scope, name = key.split(":", 1)
        if scope == "absolute":
            if not Path(name).is_absolute():
                raise InputError("An absolute input path must be absolute")
            return Path(name)
        base = {"workspace": self.workspace, "target": self.target}.get(scope)
        if base is None or Path(name).is_absolute() or ".." in Path(name).parts:
            raise InputError("An input path escapes its declared root")
        return base / name

    def build_unit_fingerprints(self) -> list[Path]:
        """Return every 1.100 fingerprint directory under the selected roots.

        Walk `<profile>/build` one level.
        A package named `build` stays a package.
        A package name can end in 16 hex characters.
        A directory with a 16-hex child is a package.
        A fingerprint inside a unit `out` directory is not included.
        An unrecognized child of a top-level package is an error.
        """
        roots = self.profiles if self.profiles is not None else (self.target,)
        found: list[Path] = []
        for root in roots:
            if not root.exists():
                continue
            if self.profiles is not None:
                # An explicit profile that is the target, or that sits inside a unit
                # out directory, is not a Cargo profile. Do not emit its units.
                if not profile_inside_target(self.target, root) or inside_unit_out(self.target, root):
                    continue
                build = root / "build"
                builds = [build] if build.is_dir() and not inside_unit_out(self.target, build) else []
            else:
                builds = [path for path in root.rglob("build") if is_profile_build_directory(self.target, path)]
            for build in builds:
                for package in build.iterdir():
                    if not package.is_dir():
                        continue
                    for child in package.iterdir():
                        fingerprint = build_unit_child(child)
                        if fingerprint is not None:
                            found.append(fingerprint)
        return found


@dataclass(frozen=True)
class Unit:
    key: str
    directory: Path
    stem: str
    kind: Kind
    metadata: dict
    cargo_hash: int

    @property
    def profile(self) -> Path:
        return fingerprint_profile(self.directory)

    @property
    def suffix(self) -> str:
        return self.directory.parent.name

    def fingerprint_digest(self) -> str:
        return sha((self.directory / self.stem).read_bytes() + (self.directory / (self.stem + ".json")).read_bytes())

    def built_after(self, marker_ns: int) -> bool:
        # Only Cargo's committed fingerprint counts. Our record never counts.
        return (self.directory / self.stem).stat().st_mtime_ns > marker_ns


def is_build_unit_package(path: Path) -> bool:
    """Return true when `path` is one Cargo 1.100 package directory.

    A package directory contains a 16-hex unit directory.
    That unit directory does not contain another 16-hex directory.
    A package whose name is 16 hex characters still contains the unit directory.
    """
    if not path.is_dir():
        return False
    saw_unit = False
    for child in path.iterdir():
        if not child.is_dir() or not UNIT_ID.fullmatch(child.name):
            continue
        if any(entry.is_dir() and UNIT_ID.fullmatch(entry.name) for entry in child.iterdir()):
            return False
        saw_unit = True
    return saw_unit


def is_profile_build_directory(target: Path, path: Path) -> bool:
    """Return true for `<profile>/build`.

    A package named `build` is not that directory.
    A profile named `build` is not that directory.
    A directory inside a unit `out` directory is not that directory.
    """
    if not path.is_dir() or path.name != "build" or not profile_inside_target(target, path.parent):
        return False
    if inside_unit_out(target, path) or is_build_unit_package(path):
        return False
    inner = path / "build"
    if (
        inner.is_dir()
        and not inside_unit_out(target, inner)
        and not is_build_unit_package(inner)
        and any(is_build_unit_package(child) for child in inner.iterdir())
    ):
        return False
    return True


def fingerprint_profile(directory: Path) -> Path:
    """Return the profile directory that owns one fingerprint directory."""
    return directory.parents[3]


def build_unit_child(child: Path) -> Path | None:
    """Return one build-unit fingerprint.

    If the unit has no fingerprint directory, return None.
    A child must be a 16-hex directory.
    A `fingerprint` path that exists and is not a directory is an error.
    """
    if not child.is_dir() or not UNIT_ID.fullmatch(child.name):
        raise InputError(f"Unknown build entry: {child}")
    fingerprint = child / "fingerprint"
    if not fingerprint.exists():
        return None
    if not fingerprint.is_dir():
        raise InputError(f"A build-unit fingerprint is not a directory: {fingerprint}")
    return fingerprint


def read_unit(layout: Layout, directory: Path) -> Unit | None:
    """Read a committed Cargo fingerprint, or return None for an unfinished unit.

    Cargo creates the directory before the first build and clears the hash
    before a rebuild. Cargo writes the hash before it writes the metadata.
    If the metadata exists, it must be readable and valid.
    """
    metadata = []
    unknown = []
    for entry in directory.iterdir():
        name = entry.name
        if name.endswith(".json") and name != RECORD:
            metadata.append(entry)
        elif name not in ("invoked.timestamp", RECORD) and not name.startswith(("dep-", "output-")):
            unknown.append(entry)
    if len(metadata) > 1:
        raise InputError(f"A fingerprint directory must contain one metadata file: {directory}")
    if not metadata:
        if not unknown or (len(unknown) == 1 and empty_fingerprint_hash(unknown[0])):
            return None
        raise InputError(f"Unknown fingerprint entry: {unknown[0]}")

    path = metadata[0]
    try:
        data = json.loads(path.read_text())
        short = (directory / path.stem).read_text()
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise InputError(f"A fingerprint directory is unreadable: {directory}") from error
    if not isinstance(data, dict) or not isinstance(data.get("deps"), list):
        raise InputError(f"A fingerprint metadata file has no dependency list: {path}")
    if short == "":
        return None
    if not UNIT_ID.fullmatch(short):
        raise InputError(f"A fingerprint hash file must contain 16 hex characters: {directory / path.stem}")
    value = int.from_bytes(bytes.fromhex(short), "little")
    kind = Kind.RUN if path.stem.startswith("run-build-script-") else Kind.COMPILE
    key = directory.relative_to(layout.target).as_posix()
    return Unit(key, directory, path.stem, kind, data, value)


@dataclass(frozen=True)
class UnitInventory:
    committed: dict[str, Unit]
    uncommitted: dict[str, Path]


def scan_units(layout: Layout) -> UnitInventory:
    """Classify every Cargo 1.100 fingerprint outside unit output directories."""
    committed: dict[str, Unit] = {}
    uncommitted: dict[str, Path] = {}
    for directory in layout.build_unit_fingerprints():
        key = directory.relative_to(layout.target).as_posix()
        if key in committed or key in uncommitted:
            raise InputError(f"Two fingerprint directories use one unit key: {key}")
        unit = read_unit(layout, directory)
        if unit is None:
            uncommitted[key] = directory
        else:
            committed[key] = unit
    return UnitInventory(committed, uncommitted)


def inventory(layout: Layout) -> dict[str, Unit]:
    """Return the committed units for the selected profiles."""
    return scan_units(layout).committed


def dependency_keys(unit: Unit, units: dict[str, Unit]) -> tuple[str, ...]:
    result: set[str] = set()
    for dependency in unit.metadata["deps"]:
        candidates = [other for other in units.values() if other.cargo_hash == dependency[3]]
        if not candidates:
            raise InputError(f"A dependency of {unit.key} has no unit record")
        # Cargo can share a fingerprint across profiles. Bind every match rather
        # than guess which profile produced the dependency. Extra edges only
        # invalidate more units; they cannot hide a changed dependency.
        result.update(candidate.key for candidate in candidates)
    return tuple(sorted(result))


def unescape_env(value: str) -> str:
    output = []
    index = 0
    while index < len(value):
        character = value[index]
        index += 1
        if character == "\\":
            if index == len(value) or value[index] not in "\\nr":
                raise InputError("Unknown rustc environment escape")
            character = {"\\": "\\", "n": "\n", "r": "\r"}[value[index]]
            index += 1
        output.append(character)
    return "".join(output)


def rustc_dep_info(path: Path, cwd: Path) -> tuple[list[Path], dict[str, str | None]]:
    files: list[Path] | None = None
    environment: dict[str, str | None] = {}
    for line in path.read_text().splitlines():
        if line.startswith("# env-dep:"):
            name, separator, value = line[10:].partition("=")
            environment[unescape_env(name)] = sha(unescape_env(value).encode()) if separator else None
        elif files is None and ": " in line:
            words = iter(line.split(": ", 1)[1].split())
            files = []
            for word in words:
                while word.endswith("\\"):
                    word = word[:-1] + " " + next(words)
                files.append(Path(os.path.abspath(cwd / word)))
    if not files:
        raise InputError(f"No rustc input paths in {path}")
    return files, environment


def encoded_environment(path: Path) -> dict[str, str | None]:
    """Read the environment dependencies from the Cargo 1.100.0 dep-info file."""
    data = memoryview(path.read_bytes())
    offset = 0

    def take(size: int) -> bytes:
        nonlocal offset
        if size < 0 or offset + size > len(data):
            raise InputError("Truncated encoded dep-info")
        result = bytes(data[offset:offset + size])
        offset += size
        return result

    def number() -> int:
        return struct.unpack("<I", take(4))[0]

    def text() -> str:
        return take(number()).decode("utf-8")

    if take(6) != b"\x01\x00\x00\x00\xff\x01":
        raise InputError("Unsupported Cargo encoded dep-info version")
    for _ in range(number()):
        if take(1) not in (b"\x00", b"\x01"):
            raise InputError("Unknown encoded path kind")
        take(number())
        checksum = take(1)
        if checksum == b"\x01":
            take(8)
            take(number())
        elif checksum != b"\x00":
            raise InputError("Unknown encoded checksum flag")
    result: dict[str, str | None] = {}
    for _ in range(number()):
        name = text()
        exists = take(1)
        if exists not in (b"\x00", b"\x01"):
            raise InputError("Unknown encoded environment flag")
        result[name] = sha(text().encode()) if exists == b"\x01" else None
    if offset != len(data):
        raise InputError("Unexpected encoded dep-info data")
    return result


def file_snapshot(path: Path, cache: dict[Path, dict] | None = None) -> dict:
    # Hash both the link identity and its current contents.
    if cache is not None and path in cache:
        return cache[path]
    if not path.is_file():
        raise InputError(f"An input is not a regular file: {path}")
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    value = {"sha256": digest.hexdigest(),
             "link": os.readlink(path) if path.is_symlink() else None}
    if cache is not None:
        cache[path] = value
    return value


def expand_directory(path: Path, target: Path, package_default: bool = False) -> list[Path]:
    files: list[Path] = []
    visited: set[Path] = set()

    def walk(directory: Path) -> None:
        real = directory.resolve()
        if real in visited:
            raise InputError("A declared input directory contains a symlink cycle")
        visited.add(real)
        for child in sorted(directory.iterdir()):
            if package_default and (child.name == ".git" or child.resolve().is_relative_to(target)):
                continue
            if child.is_dir():
                walk(child)
            elif child.is_file():
                files.append(child)
            else:
                raise InputError(f"A declared input is not a regular file: {child}")
        visited.remove(real)
    walk(path)
    return files


def directory_snapshot(layout: Layout, root: Path, package_default: bool = False) -> dict:
    files = expand_directory(root, layout.target, package_default)
    # Include empty directories and directory symlinks, not only regular files.
    entries: dict[str, str | None] = {}
    visited: set[Path] = set()

    def walk(path: Path) -> None:
        real = path.resolve()
        if real in visited:
            raise InputError("A declared input directory contains a symlink cycle")
        visited.add(real)
        entries[layout.path_key(path)] = os.readlink(path) if path.is_symlink() else None
        for child in sorted(path.iterdir()):
            if package_default and (child.name == ".git" or child.resolve().is_relative_to(layout.target)):
                continue
            if child.is_dir():
                walk(child)
        visited.remove(real)
    walk(root)
    return {"files": sorted(layout.path_key(path) for path in files), "directories": entries,
            "package_default": package_default}


def load_record(unit: Unit) -> dict:
    envelope = json.loads((unit.directory / RECORD).read_text())
    data = envelope["data"]
    if envelope.keys() != {"data", "sha256"} or envelope["sha256"] != sha(canonical(data)) or data["version"] != VERSION:
        raise InputError("Damaged or unsupported input record")
    if data["unit"] != unit.key or data["kind"] != unit.kind.value or data["cargo"] != unit.fingerprint_digest():
        raise InputError("An input record belongs to a different unit fingerprint")
    if set(data) != {"version", "unit", "kind", "cargo", "package", "files", "directories", "env", "cargo_env", "dependencies"}:
        raise InputError("An input record has unexpected fields")
    for field in ("files", "directories", "env", "cargo_env", "dependencies"):
        if not isinstance(data[field], dict):
            raise InputError("An input record has an invalid field type")
    return data


def inputs_match(layout: Layout, record: dict, snapshots: dict[Path, dict] | None = None) -> bool:
    try:
        for key, value in record["files"].items():
            if not HASH.fullmatch(value["sha256"]) or file_snapshot(layout.path(key), snapshots) != value:
                return False
        for key, members in record["directories"].items():
            current = directory_snapshot(layout, layout.path(key), members["package_default"])
            if current != members:
                return False
        return True
    except (OSError, KeyError, ValueError, TypeError, InputError):
        return False


def record_signature(record: dict) -> str:
    return sha(canonical(record))


def evict_unit_directory(directory: Path) -> None:
    # The unit directory holds the fingerprint, compiler output, and script output.
    remove(directory.parent)
    # CI disables incremental output because its session hashes do not name units.
    remove(fingerprint_profile(directory) / "incremental")


def profile_directories(layout: Layout, scanned: UnitInventory) -> set[Path]:
    """Return every profile directory that eviction must clean.

    A committed or unfinished unit identifies its profile. An explicit
    profile can also exist without a fingerprint. Unit output directories
    can contain nested profiles, which belong to the outer unit.
    """
    profiles = {unit.profile for unit in scanned.committed.values()}
    profiles.update(fingerprint_profile(directory) for directory in scanned.uncommitted.values())
    if layout.profiles is not None:
        profiles.update(
            path for path in layout.profiles
            if path.is_dir() and profile_inside_target(layout.target, path) and not inside_unit_out(layout.target, path))
    return profiles


def discard_unrecorded(layout: Layout, scanned: UnitInventory, retained: set[str], profiles: set[Path]) -> None:
    """Remove output that no retained unit in the same profile owns."""
    for directory in scanned.uncommitted.values():
        evict_unit_directory(directory)
    hashes_by_profile: dict[Path, set[str]] = {}
    for key, unit in scanned.committed.items():
        if key not in retained:
            evict_unit_directory(unit.directory)
        else:
            hashes_by_profile.setdefault(unit.profile, set()).add(unit.suffix)
    for profile in profiles:
        if not profile.is_dir() or not profile_inside_target(layout.target, profile):
            continue
        if inside_unit_out(layout.target, profile):
            continue
        valid_hashes = hashes_by_profile.get(profile, set())
        deps = profile / "deps"
        if deps.is_dir():
            for path in list(deps.iterdir()):
                match = UNIT_HASH.search(path.name)
                if match is None or match[1] not in valid_hashes:
                    remove(path)
        examples = profile / "examples"
        if examples.is_dir():
            for path in list(examples.iterdir()):
                match = UNIT_HASH.search(path.name)
                if match and match[1] not in valid_hashes:
                    remove(path)
        build = profile / "build"
        if not build.is_dir():
            continue
        for path in list(build.iterdir()):
            if path.is_dir():
                retain_build_unit_children(layout, path, retained)
            else:
                remove(path)


def retain_build_unit_children(layout: Layout, package: Path, retained: set[str]) -> None:
    """If its fingerprint is a retained unit, keep that child."""
    for child in list(package.iterdir()):
        kept = False
        if child.is_dir() and UNIT_ID.fullmatch(child.name):
            fingerprint = child / "fingerprint"
            if fingerprint.is_dir():
                key = fingerprint.relative_to(layout.target).as_posix()
                kept = key in retained
        if not kept:
            remove(child)


def scrub_outputs(layout: Layout, profiles: set[Path]) -> None:
    for profile in profiles:
        if not profile.is_dir() or not profile_inside_target(layout.target, profile):
            continue
        if inside_unit_out(layout.target, profile):
            continue
        for entry in profile.iterdir():
            if entry.name not in ("deps", "build", "examples", ".cargo-lock"):
                remove(entry)
        # Unhashed example aliases also carry no unit identity.
        examples = profile / "examples"
        if examples.exists():
            for entry in examples.iterdir():
                if not UNIT_HASH.search(entry.name):
                    remove(entry)
    if layout.profiles is not None:
        return

    def walk(directory: Path) -> None:
        for path in directory.iterdir():
            if path in profiles:
                continue
            if any(profile.is_relative_to(path) for profile in profiles):
                walk(path)
            elif path.name not in (".rustc_info.json", "CACHEDIR.TAG"):
                remove(path)
    if layout.target.exists():
        walk(layout.target)


def validate_records(layout: Layout, units: dict[str, Unit], records: dict[str, dict]) -> set[str]:
    bad: set[str] = set(units) - set(records)
    snapshots: dict[Path, dict] = {}
    for key, record in records.items():
        if not inputs_match(layout, record, snapshots):
            bad.add(key)
        for child, signature in record["dependencies"].items():
            if child not in records or record_signature(records[child]) != signature:
                bad.add(key)
    changed = True
    while changed:
        changed = False
        for key, record in records.items():
            if key not in bad and any(child in bad for child in record["dependencies"]):
                bad.add(key)
                changed = True
    return bad


def normalize_checkout(workspace: Path) -> None:
    paths = subprocess.check_output(["git", "ls-files", "--recurse-submodules", "-z"], cwd=workspace).split(b"\0")
    directories: set[Path] = set()
    for raw in paths:
        if not raw:
            continue
        path = workspace / os.fsdecode(raw)
        if path.is_file() or path.is_symlink():
            os.utime(path, ns=(OLD_TIME_NS, OLD_TIME_NS), follow_symlinks=False)
        parent = path.parent
        while parent != workspace and parent.is_relative_to(workspace):
            if parent.is_dir():
                directories.add(parent)
            parent = parent.parent
    for path in directories:
        os.utime(path, ns=(OLD_TIME_NS, OLD_TIME_NS))


def restore(layout: Layout, normalize: bool = True) -> dict[str, int]:
    scanned = scan_units(layout)
    units = scanned.committed
    profiles = profile_directories(layout, scanned)
    records: dict[str, dict] = {}
    for key, unit in units.items():
        try:
            records[key] = load_record(unit)
        except (OSError, KeyError, ValueError, TypeError, InputError):
            pass
    bad = validate_records(layout, units, records)
    discard_unrecorded(layout, scanned, set(units) - bad, profiles)
    # Direct execution must never find a stale uplifted binary or DX bundle.
    scrub_outputs(layout, profiles)
    if normalize:
        normalize_checkout(layout.workspace)
    return {"retained": len(units) - len(bad), "invalidated": len(bad)}


def checkout_snapshot(workspace: Path) -> dict:
    files = subprocess.check_output(["git", "ls-files", "--recurse-submodules", "-z"], cwd=workspace).split(b"\0")
    return {os.fsdecode(path): file_snapshot(workspace / os.fsdecode(path)) for path in files
            if path and (workspace / os.fsdecode(path)).is_file()}


def begin(layout: Layout, session: Path) -> None:
    session.mkdir(parents=True, exist_ok=False)
    marker = session / "marker"
    marker.touch()
    atomic_json(session / "session.json", {"workspace": str(layout.workspace), "target": str(layout.target),
                                         "marker_ns": marker.stat().st_mtime_ns,
                                         "checkout": checkout_snapshot(layout.workspace)})


def journals(session: Path) -> tuple[dict[str, dict], dict[str, set[str]], set[str]]:
    descriptors: dict[str, dict] = {}
    reads: dict[str, set[str]] = {}
    writes: set[str] = set()
    for path in session.glob("trace-*.jsonl"):
        completed = False
        with path.open() as stream:
            for line in stream:
                event = json.loads(line)
                if event["event"] == "unit":
                    descriptors[event["unit"]] = event
                elif event["event"] == "open":
                    if event["write"]:
                        writes.add(event["path"])
                    if event.get("read", not event["write"]) and event["unit"]:
                        reads.setdefault(event["unit"], set()).add(event["path"])
                elif event["event"] == "complete":
                    completed = event["errors"] == 0
        if not completed:
            raise InputError("The input trace did not finish without errors")
    return descriptors, reads, writes


def build_script_output(unit: Unit) -> Path:
    """Return the file that stores the build script instructions for one run unit."""
    path = unit.directory.parent / "run" / "stdout"
    if not path.is_file():
        raise InputError(f"A build script unit has no output file: {unit.key}")
    return path


def unit_package_name(unit: Unit) -> str:
    """Return the package name stored in one unit path."""
    return unit.directory.parent.parent.name


def manifest_package_name(path: Path) -> str:
    """Return the package name from one Cargo.toml."""
    try:
        data = tomllib.loads(path.read_text())
    except (OSError, UnicodeError, tomllib.TOMLDecodeError) as error:
        raise InputError(f"A unit manifest is unreadable: {path}") from error
    package = data.get("package") if isinstance(data, dict) else None
    name = package.get("name") if isinstance(package, dict) else None
    if not isinstance(name, str) or not name:
        raise InputError(f"A unit manifest has no package name: {path}")
    return name


def make_record(layout: Layout, unit: Unit, descriptor: dict, snapshots: dict[Path, dict] | None = None) -> dict:
    package = Path(descriptor["package"])
    paths: list[Path] = [package / "Cargo.toml"]
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo")).absolute()
    if package.is_relative_to(layout.workspace) and not package.is_relative_to(cargo_home) and not package.is_relative_to(layout.target):
        # Cargo's manifests, lockfile, and configuration are implicit unit inputs.
        paths.extend(path for path in [layout.workspace / name for name in (
            "Cargo.toml", "Cargo.lock", ".cargo/config.toml", ".cargo/config", "rust-toolchain.toml", "rust-toolchain")]
                     if path.is_file())
    directories: dict[str, dict] = {}
    env: dict[str, str | None] = {}
    cargo_env: dict[str, str | None] = {}
    if unit.kind == Kind.COMPILE:
        source, env = rustc_dep_info(Path(descriptor["dep_info"]), Path(descriptor["cwd"]))
        paths.extend(source)
        cargo_env = encoded_environment(unit.directory / ("dep-" + unit.stem))
    else:
        output = build_script_output(unit).read_text()
        declared = [line.split("=", 1)[1] for line in output.splitlines()
                    if line.startswith(("cargo:rerun-if-changed=", "cargo::rerun-if-changed="))]
        package_default = not declared
        if package_default:
            declared = [str(package)]
        for value in declared:
            path = Path(os.path.abspath(package / value))
            if path.is_dir():
                files = expand_directory(path, layout.target, package_default)
                directories[layout.path_key(path)] = directory_snapshot(layout, path, package_default)
                paths.extend(files)
            else:
                paths.append(path)
        for local in unit.metadata["local"]:
            if "RerunIfEnvChanged" in local:
                entry = local["RerunIfEnvChanged"]
                env[entry["var"]] = None if entry["val"] is None else sha(entry["val"].encode())
    files = {layout.path_key(path): file_snapshot(path, snapshots) for path in paths}
    return {"version": VERSION, "unit": unit.key, "kind": unit.kind.value,
            "cargo": unit.fingerprint_digest(), "package": layout.path_key(package),
            "files": files, "directories": directories, "env": env,
            "cargo_env": cargo_env, "dependencies": {}}


def check_reads(layout: Layout, unit: Unit, record: dict, reads: set[str]) -> None:
    declared = {layout.path(key).resolve() for key in record["files"]}
    for name in reads:
        path = Path(name)
        # Registry sources, system files, and owned build outputs are outside scope.
        if not path.is_relative_to(layout.workspace) or path.is_relative_to(layout.target):
            continue
        cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
        if path.is_relative_to(cargo_home / "registry" / "src"):
            continue
        if not path.exists():
            raise InputError(f"An input disappeared during the build of {unit.key}: {path}")
        if path.is_file() and path.resolve() not in declared:
            raise InputError(f"Undeclared input read by {unit.key}: {path.relative_to(layout.workspace)}")


def save(layout: Layout, session: Path, scrub: bool = True) -> dict[str, int]:
    state = json.loads((session / "session.json").read_text())
    marker_ns = state["marker_ns"]
    if state["checkout"] != checkout_snapshot(layout.workspace):
        raise InputError("The checkout changed while Cargo built its units")
    descriptors, reads, writes = journals(session)
    tracked = subprocess.check_output(["git", "ls-files", "--recurse-submodules", "-z"], cwd=layout.workspace).split(b"\0")
    tracked_paths = {str((layout.workspace / os.fsdecode(path)).resolve()) for path in tracked if path}
    if writes & tracked_paths:
        raise InputError("A build changed a tracked checkout input")
    scanned = scan_units(layout)
    units = scanned.committed
    for key in descriptors:
        if key not in units and key not in scanned.uncommitted:
            raise InputError(f"The trace recorded a unit that inventory did not: {key}")
    profiles = profile_directories(layout, scanned)
    records: dict[str, dict] = {}
    built: set[str] = set()
    snapshots: dict[Path, dict] = {}
    unknown: set[str] = set()
    for key, unit in units.items():
        if unit.built_after(marker_ns):
            if key not in descriptors:
                unknown.add(key)
                continue
            descriptor = descriptors[key]
            trace_kind = descriptor.get("kind")
            if trace_kind != unit.kind.value:
                raise InputError(
                    f"The trace kind {trace_kind} does not match the fingerprint kind {unit.kind.value}: {key}")
            manifest = Path(descriptor["package"]) / "Cargo.toml"
            manifest_name = manifest_package_name(manifest)
            path_name = unit_package_name(unit)
            if manifest_name != path_name:
                raise InputError(
                    f"The manifest package {manifest_name} does not match the target package {path_name}: {key}")
            try:
                record = make_record(layout, unit, descriptor, snapshots)
            except (OSError, KeyError, ValueError, StopIteration, InputError) as error:
                package = Path(descriptor["package"])
                cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo")).absolute()
                if package.is_relative_to(layout.workspace) and not package.is_relative_to(cargo_home):
                    raise InputError(f"Cannot record the input contract for {key}: {error}") from error
                unknown.add(key)
                continue
            check_reads(layout, unit, record, reads.get(key, set()))
            records[key] = record
            built.add(key)
        else:
            try:
                records[key] = load_record(unit)
            except (OSError, KeyError, ValueError, TypeError, InputError):
                unknown.add(key)
    active: set[str] = set()
    finished: set[str] = set()

    def finish(key: str) -> None:
        if key in finished:
            return
        if key in active or key not in records:
            raise InputError("A unit input record has an unresolved dependency")
        active.add(key)
        try:
            if key in built:
                dependencies = dependency_keys(units[key], units)
                for child in dependencies:
                    finish(child)
                records[key]["dependencies"] = {child: record_signature(records[child]) for child in dependencies}
            finished.add(key)
        finally:
            active.remove(key)
    for key in list(records):
        try:
            finish(key)
        except (InputError, KeyError):
            unknown.add(key)
    for key in unknown:
        records.pop(key, None)
    bad = validate_records(layout, units, records)
    discard_unrecorded(layout, scanned, set(units) - bad, profiles)
    for key, record in records.items():
        if key not in bad and key in built:
            atomic_json(units[key].directory / RECORD, {"data": record, "sha256": record_signature(record)})
    if scrub:
        scrub_outputs(layout, profiles)
    return {"built": len(built), "recorded": len(built - bad), "discarded": len(bad),
            "retained": len(units) - len(bad),
            "dx_units": sum(bool(descriptors.get(key, {}).get("via_dx")) for key in built - bad)}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("begin", "restore", "save", "trace"))
    parser.add_argument("--workspace", type=Path, default=Path.cwd())
    parser.add_argument("--target", type=Path, default=os.environ.get("CARGO_TARGET_DIR"))
    parser.add_argument("--session", type=Path, default=os.environ.get("JBOTCI_INPUT_SESSION"))
    parser.add_argument("command", nargs=argparse.REMAINDER)
    arguments = parser.parse_args()
    if arguments.target is None:
        parser.error("Set CARGO_TARGET_DIR or pass --target before the mode")
    if arguments.mode != "restore" and arguments.session is None:
        parser.error("Set JBOTCI_INPUT_SESSION or pass --session before the mode")
    target = arguments.target
    if arguments.mode == "trace":
        target = Path(json.loads((arguments.session / "session.json").read_text())["target"])
    layout = Layout(arguments.workspace.resolve(), target.resolve())
    if arguments.mode == "begin":
        begin(layout, arguments.session)
    elif arguments.mode == "restore":
        print(json.dumps(restore(layout)))
    elif arguments.mode == "save":
        print(json.dumps(save(layout, arguments.session)))
    else:
        command = arguments.command
        if command and command[0] == "--":
            command = command[1:]
        return trace(layout.workspace, layout.target, arguments.session / f"trace-{uuid4().hex}.jsonl", command)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (InputError, TraceError, OSError, ValueError, KeyError) as error:
        print(f"Cargo input cache: {error}", file=__import__("sys").stderr)
        raise SystemExit(1)
