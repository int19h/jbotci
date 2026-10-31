"""Collect Cargo units, prune unchecked artifacts, and stamp successful caches.

CI installs a private Cargo proxy. It preserves JSON for tools such as dx and
renders compiler messages for human callers. A compiler guard rejects a save
if a Cargo process bypasses the proxy. The proxy never changes build results.
"""
from __future__ import annotations

from enum import Enum
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
from uuid import uuid4


class CacheError(Exception):
    """The selected Cargo units cannot prove that the cache is safe."""


class Command(Enum):
    BUILD = "build"
    CHECK = "check"
    TEST = "test"
    RUN = "run"
    RUSTC = "rustc"
    BENCH = "bench"
    INSTALL = "install"


READ_ONLY = {"metadata", "tree", "locate-project", "pkgid", "read-manifest", "fetch", "generate-lockfile", "--version", "-V", "--help", "-h", "version"}
HASH = re.compile(r"-([0-9a-f]{16})(?:\.|$)")
SCRIPT = Path(__file__).resolve()


def state_dir() -> Path:
    return Path(os.environ["JBOTCI_CARGO_CACHE_STATE"]).resolve()


def write_json(path: Path, value: object) -> None:
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_text(json.dumps(value) + "\n")
    temporary.replace(path)


def reject(reason: str) -> None:
    write_json(state_dir() / f"rejected-{uuid4().hex}.json", {"reason": reason})


def install(directory: Path, uncached_targets: tuple[Path, ...] = ()) -> dict[str, str]:
    directory.mkdir(parents=True, exist_ok=False)
    real = subprocess.check_output(["rustup", "which", "cargo"], text=True).strip()
    # Cargo uses CARGO for nested commands when its executable is not named cargo.
    shutil.copy2(real, directory / "cargo-real")
    for name, mode in (("cargo", "cargo"), ("rustc-guard", "rustc")):
        path = directory / name
        path.write_text(f"#!/bin/sh\nexec python3 {shlex_quote(str(SCRIPT))} {mode} \"$@\"\n")
        path.chmod(0o755)
    environment = {
        "JBOTCI_CARGO_CACHE_STATE": str(directory),
        "JBOTCI_CARGO_REAL": str(directory / "cargo-real"),
        "JBOTCI_CARGO_UNCACHED_TARGETS": json.dumps([str(path.resolve()) for path in uncached_targets]),
        "RUSTUP_TOOLCHAIN": subprocess.check_output(["rustup", "show", "active-toolchain"], text=True).split()[0],
        "RUSTC": subprocess.check_output(["rustup", "which", "rustc"], text=True).strip(),
        "RUSTDOC": subprocess.check_output(["rustup", "which", "rustdoc"], text=True).strip(),
        "JBOTCI_CARGO_PREVIOUS_WRAPPER": os.environ.get("RUSTC_WRAPPER", ""),
        "CARGO": str(directory / "cargo"),
        "RUSTC_WRAPPER": str(directory / "rustc-guard"),
        # Incremental directories use rustc session hashes, not Cargo unit hashes.
        "CARGO_INCREMENTAL": "0",
    }
    write_json(directory / "environment.json", environment)
    return environment


def shlex_quote(value: str) -> str:
    import shlex
    return shlex.quote(value)


def process_start(pid: int) -> str:
    # Linux keeps this start tick across exec and changes it when a PID is reused.
    return Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()[19]


def exec_cargo(arguments: list[str]) -> None:
    identifier, *command = arguments
    write_json(state_dir() / f"pid-{os.getpid()}.json", {"id": identifier, "start": process_start(os.getpid())})
    os.execv(os.environ["JBOTCI_CARGO_REAL"], [os.environ["JBOTCI_CARGO_REAL"], *command])


def guard(arguments: list[str]) -> None:
    parent = os.getppid()
    report = state_dir() / f"pid-{parent}.json"
    if not report.is_file() or json.loads(report.read_text())["start"] != process_start(parent):
        reject(f"Compiler parent {parent} did not report its Cargo invocation")
    previous = os.environ.get("JBOTCI_CARGO_PREVIOUS_WRAPPER")
    command = [previous, *arguments] if previous else arguments
    os.execvp(command[0], command)


def fingerprint_for_file(path: Path, root: Path) -> Path:
    if not path.is_relative_to(root) or not path.is_file():
        raise CacheError(f"Artifact is missing or outside the cached target directory: {path}")
    for parent in path.parents:
        fingerprints = parent / ".fingerprint"
        if not fingerprints.is_dir():
            continue
        # custom-build executables live in build/<unit>/ rather than deps/.
        if path.parent.parent == parent / "build":
            candidate = fingerprints / path.parent.name
            if candidate.is_dir():
                return candidate
        match = HASH.search(path.name)
        if path.parent in (parent / "deps", parent / "examples") and match:
            candidates = set(fingerprints.glob(f"*-{match[1]}"))
            if len(candidates) == 1:
                return candidates.pop()
        candidates: set[Path] = set()
        for artifact in [*(parent / "deps").glob("*"), *(parent / "examples").glob("*")]:
            match = HASH.search(artifact.name)
            if match and artifact.is_file() and os.path.samestat(path.stat(), artifact.stat()):
                candidates.update(fingerprints.glob(f"*-{match[1]}"))
        if len(candidates) == 1:
            return candidates.pop()
        raise CacheError(f"Artifact does not identify exactly one Cargo unit: {path}: {sorted(map(str, candidates))}")
    raise CacheError(f"Artifact has no supported Cargo fingerprint directory: {path}")


def cargo(arguments: list[str]) -> int:
    identifier = uuid4().hex
    record = state_dir() / f"invocation-{identifier}.json"
    command = list(arguments)
    # Cargo passes its subcommand name to external cargo-* programs only.
    subcommand = next((arg for arg in command if not arg.startswith("+")), "")
    compiling = subcommand in {item.value for item in Command}
    passthrough = any(arg.startswith("--message-format") and "json" in arg for arg in command)
    if compiling:
        if any(arg.startswith("--message-format") for arg in command) and not passthrough:
            reject(f"Cargo invocation cannot report JSON: {arguments!r}")
        elif not passthrough:
            position = command.index("--") if "--" in command else len(command)
            command.insert(position, "--message-format=json")
    elif subcommand not in READ_ONLY:
        reject(f"Unsupported Cargo invocation: {arguments!r}")
    root = Path(os.environ["CARGO_TARGET_DIR"]).resolve()
    for index, argument in enumerate(command):
        if argument == "--target-dir":
            root = Path(command[index + 1]).resolve()
        elif argument.startswith("--target-dir="):
            root = Path(argument.split("=", 1)[1]).resolve()
    write_json(record, {"arguments": arguments, "target": str(root), "complete": False})
    process = subprocess.Popen([sys.executable, str(SCRIPT), "exec-cargo", identifier, *command], stdout=subprocess.PIPE)
    messages: list[dict] = []
    units: set[str] = set()
    aliases: set[str] = set()
    finished = not compiling
    success = not compiling
    assert process.stdout is not None
    for raw in process.stdout:
        message = None
        if compiling and not finished:
            try:
                message = json.loads(raw)
            except (ValueError, UnicodeError):
                pass
        if isinstance(message, dict) and "reason" in message:
            reason = message["reason"]
            if reason in ("compiler-artifact", "build-script-executed"):
                messages.append(message)
                try:
                    if reason == "build-script-executed":
                        out = Path(message["out_dir"])
                        unit = out.parent.parent.parent / ".fingerprint" / out.parent.name
                        if not unit.is_relative_to(root) or not unit.is_dir():
                            raise CacheError(f"Build-script run has no fingerprint: {out}")
                        units.add(str(unit))
                    else:
                        for filename in message["filenames"]:
                            path = Path(filename)
                            units.add(str(fingerprint_for_file(path, root)))
                            aliases.add(str(path))
                except CacheError as error:
                    reject(str(error))
            elif reason == "build-finished":
                finished = True
                success = message["success"]
            elif reason != "compiler-message":
                reject(f"Unsupported Cargo JSON message: {reason}")
            if passthrough:
                sys.stdout.buffer.write(raw)
            elif reason == "compiler-message":
                sys.stderr.write(message["message"].get("rendered") or "")
        else:
            sys.stdout.buffer.write(raw)
        sys.stdout.flush()
    status = process.wait()
    write_json(record, {"arguments": arguments, "target": str(root), "complete": finished,
                        "success": success and status == 0, "units": sorted(units),
                        "aliases": sorted(aliases), "messages": messages})
    return status


def remove(path: Path) -> None:
    if path.is_dir() and not path.is_symlink():
        shutil.rmtree(path)
    else:
        path.unlink(missing_ok=True)


def selected_units(root: Path, state: Path) -> tuple[set[Path], set[Path], int]:
    rejected = list(state.glob("rejected-*.json"))
    if rejected:
        raise CacheError("Cache save rejected: " + "; ".join(path.read_text().strip() for path in rejected))
    records = [json.loads(path.read_text()) for path in state.glob("invocation-*.json")]
    if not records or any(not record["complete"] for record in records):
        raise CacheError("Every Cargo invocation must finish and report its units before a cache save")
    units: set[Path] = set()
    aliases: set[Path] = set()
    environment_path = state / "environment.json"
    environment = json.loads(environment_path.read_text()) if environment_path.is_file() else {}
    uncached = [Path(path) for path in json.loads(environment.get("JBOTCI_CARGO_UNCACHED_TARGETS", "[]"))]
    if any(path.is_relative_to(root) or root.is_relative_to(path) for path in uncached):
        raise CacheError("An uncached target directory must be separate from the saved target directory")
    for record in records:
        target = Path(record["target"])
        if not target.is_relative_to(root):
            if not any(target.is_relative_to(path) for path in uncached):
                raise CacheError(f"Cargo used an undeclared target directory: {target}")
            # Reports remain complete, but this directory is never saved.
            continue
        if not record["success"]:
            # Expected compile-fail tests use isolated target directories.
            if target == root:
                raise CacheError("A Cargo invocation for the cache root failed")
            remove(target)
            continue
        units.update(map(Path, record["units"]))
        aliases.update(map(Path, record["aliases"]))
    return units, aliases, len(records)


def prune_profiles(profiles: list[Path], units: set[Path], aliases: set[Path]) -> int:
    removed = 0
    for profile in profiles:
        fingerprints = profile / ".fingerprint"
        if not fingerprints.is_dir():
            continue
        retained_hashes = {unit.name.rsplit("-", 1)[-1] for unit in units if unit.parent == fingerprints}
        for unit in fingerprints.iterdir():
            if unit not in units:
                remove(unit)
                removed += 1
        for directory in ("deps", "build", "incremental", "examples"):
            container = profile / directory
            if not container.exists():
                continue
            for path in container.iterdir():
                match = HASH.search(path.name)
                if directory == "incremental" or (path not in aliases and (not match or match[1] not in retained_hashes)):
                    remove(path)
        for path in profile.iterdir():
            if path.name not in (".fingerprint", "deps", "build", "incremental", "examples", ".cargo-lock") and path not in aliases:
                remove(path)
    return removed


def prune(root: Path, state: Path) -> dict[str, int]:
    units, aliases, invocations = selected_units(root, state)
    profiles = {path.parent for path in root.rglob(".fingerprint")}
    removed = prune_profiles(list(profiles), units, aliases)
    prune_other_outputs(root, profiles)
    return {"invocations": invocations, "selected_units": len(units), "removed_units": removed}


def prune_other_outputs(directory: Path, profiles: set[Path]) -> None:
    # DX bundles and native side caches do not have a Cargo unit identity.
    # Retain only profile trees and their ancestors, plus Cargo's version data.
    if not directory.exists():
        return
    for path in directory.iterdir():
        if path in profiles:
            continue
        if any(profile.is_relative_to(path) for profile in profiles):
            prune_other_outputs(path, profiles)
        elif path.name not in (".rustc_info.json", "CACHEDIR.TAG", ".cache-commit"):
            remove(path)


def main() -> int:
    mode, *arguments = sys.argv[1:]
    if mode == "cargo":
        return cargo(arguments)
    if mode == "exec-cargo":
        exec_cargo(arguments)
    elif mode == "rustc":
        guard(arguments)
    elif mode == "install":
        environment = install(Path(arguments[0]).resolve(), tuple(Path(path) for path in arguments[1:]))
        with open(os.environ["GITHUB_ENV"], "a") as output:
            for key, value in environment.items():
                output.write(f"{key}={value}\n")
        with open(os.environ["GITHUB_PATH"], "a") as output:
            output.write(str(Path(arguments[0]).resolve()) + "\n")
    elif mode == "prune":
        root = Path(os.environ["CARGO_TARGET_DIR"]).resolve()
        print(json.dumps(prune(root, state_dir())))
    else:
        raise CacheError(f"Unsupported operation: {mode}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (CacheError, KeyError, OSError, ValueError) as error:
        print(f"Cargo cache: {error}", file=sys.stderr)
        raise SystemExit(1)
