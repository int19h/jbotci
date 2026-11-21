"""Trace build input reads without retaining command secrets or environment values."""
from __future__ import annotations

from dataclasses import dataclass, replace
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import threading
from typing import Callable


LINE = re.compile(r"\s*(\d+)(?:<([^>]*)>)?\s+(.*)")
EXIT = re.compile(r"\+\+\+ (?:exited with \d+|killed by SIG[A-Z0-9]+(?: \(core dumped\))?) \+\+\+")
TEARDOWN = re.compile(r"(?:\?\?\?\(\s*<unfinished \.\.\.>|<\.\.\. \?\?\? resumed>\)\s*=\s*\?|\?\?\?\(\)\s*=\s*\?)")


class TraceError(Exception):
    """A trace cannot establish the declared input contract."""


@dataclass
class Process:
    cwd: str
    unit: str | None = None
    via_dx: bool = False
    parent: int | None = None
    exe: str | None = None
    argument_count: int | None = None

    def identity(self) -> dict:
        return {"parent": self.parent, "exe": self.exe,
                "cmdline": {"arguments": "<redacted>", "argument_count": self.argument_count}}


def process_snapshot(pid: int) -> dict:
    """Capture diagnostic evidence while the tracee exists, without retaining arguments."""
    root = Path(f"/proc/{pid}")
    result: dict = {"source": "proc-snapshot"}
    try:
        status = dict(line.split(":", 1) for line in (root / "status").read_text().splitlines())
        result.update(parent=int(status["PPid"]), tgid=int(status["Tgid"]))
        result["exe"] = os.readlink(root / "exe")
        command = (root / "cmdline").read_bytes().rstrip(b"\0")
        result["cmdline"] = {"arguments": "<redacted>",
                             "argument_count": max(0, len(command.split(b"\0")) - 1) if command else None}
    except (OSError, KeyError, ValueError):
        pass
    return result


def operation_name(body: str) -> str:
    match = re.match(r"(?:<\.\.\. )?([a-zA-Z0-9_]+|\?\?\?)(?:\(| resumed>)", body)
    return match[1] if match else "unknown"


def string_at(text: str, start: int = 0) -> tuple[str, int]:
    start = text.index('"', start)
    index = start + 1
    value = bytearray()
    escapes = {"n": 10, "r": 13, "t": 9, "v": 11, "f": 12, "a": 7, "b": 8, "\\": 92, '"': 34}
    while index < len(text):
        character = text[index]
        index += 1
        if character == '"':
            return os.fsdecode(bytes(value)), index
        if character != "\\":
            value.extend(character.encode("utf-8", errors="surrogateescape"))
            continue
        character = text[index]
        index += 1
        if character in escapes:
            value.append(escapes[character])
        elif character in "01234567":
            digits = character
            while len(digits) < 3 and index < len(text) and text[index] in "01234567":
                digits += text[index]
                index += 1
            value.append(int(digits, 8))
        elif character == "x":
            value.append(int(text[index:index + 2], 16))
            index += 2
        else:
            raise TraceError("Unknown trace string escape")
    raise TraceError("Incomplete quoted trace string")


def array_at(text: str, start: int) -> tuple[list[str], int]:
    start = text.index("[", start)
    index = start + 1
    values: list[str] = []
    while index < len(text):
        if text[index] == '"':
            value, index = string_at(text, index)
            values.append(value)
        elif text.startswith("...", index):
            raise TraceError("An abbreviated argument cannot establish input coverage")
        elif text[index] == "]":
            return values, index + 1
        else:
            index += 1
    raise TraceError("Incomplete trace array")


def flag(arguments: list[str], name: str) -> str | None:
    for index, argument in enumerate(arguments):
        if argument == name:
            return arguments[index + 1]
        if argument.startswith(name + "="):
            return argument[len(name) + 1:]
    return None


def extra_filename(arguments: list[str]) -> str | None:
    for argument in arguments:
        if argument.startswith("extra-filename=-"):
            return argument.removeprefix("extra-filename=-")
        if argument.startswith("-Cextra-filename=-"):
            return argument.removeprefix("-Cextra-filename=-")
    return None


UNIT_ID = re.compile(r"[0-9a-f]{16}")
CLASSIC_DIR = re.compile(r"-[0-9a-f]{16}$")


def cargo_rustc_metadata(arguments: list[str]) -> bool:
    """Return true when rustc arguments carry Cargo's unit metadata hash."""
    return any(re.fullmatch(r"(?:-C)?metadata=[0-9a-f]{16}", argument) for argument in arguments)


def build_unit_fingerprint(out_dir: Path) -> Path | None:
    """Return the fingerprint directory for `<profile>/build/<package>/<16 hex>/out`."""
    if out_dir.name != "out" or not UNIT_ID.fullmatch(out_dir.parent.name):
        return None
    if len(out_dir.parents) < 3 or out_dir.parents[2].name != "build":
        return None
    return out_dir.parent / "fingerprint"


def classic_fingerprint(target: Path, out_dir: Path, package: str, suffix: str) -> Path | None:
    if out_dir.name in ("deps", "examples"):
        profile = out_dir.parent
    elif out_dir.parent.name == "build":
        profile = out_dir.parent.parent
        named = classic_package_dir(out_dir.name)
        if named is not None and named[0] != package:
            raise TraceError(f"The target package {named[0]} does not match CARGO_PKG_NAME {package}")
    else:
        return None
    if not profile_inside_target(target, profile):
        return None
    return profile / ".fingerprint" / f"{package}-{suffix}"


def normalized_path(value: str, cwd: str) -> Path:
    """Join a trace path to the process directory without following a symlink."""
    path = Path(value)
    if not path.is_absolute():
        path = Path(cwd) / path
    return Path(os.path.normpath(path))


def classic_package_dir(name: str) -> tuple[str, str] | None:
    match = CLASSIC_DIR.search(name)
    if match is None:
        return None
    package = name[:match.start()]
    if not package:
        return None
    return package, name[match.start() + 1:]


def profile_inside_target(target: Path, profile: Path) -> bool:
    return profile.is_relative_to(target) and profile != target


def classic_build_script_fingerprint(target: Path, executable: Path, out_dir: Path,
                                      cargo_package: str) -> Path | None:
    """Return the classic run fingerprint for one build-script executable.

    Cargo 1.99 runs `<profile>/build/<package>-<16 hex>/build-script-<stem>`.
    `OUT_DIR` is `<profile>/build/<package>-<16 hex>/out` for the run unit.
    Those two suffixes are different directories of one package.
    The package name in that path must equal CARGO_PKG_NAME.
    Return None when `executable` is not in that place.
    """
    if not executable.name.startswith("build-script-") or executable.name == "build-script-":
        return None
    script_dir = executable.parent
    parsed = classic_package_dir(script_dir.name)
    if parsed is None or script_dir.parent.name != "build":
        return None
    package, _compile_hash = parsed
    profile = script_dir.parent.parent
    if not profile_inside_target(target, profile):
        raise TraceError("Unknown build script unit layout")
    out_package = classic_package_dir(out_dir.parent.name)
    if (
        out_dir.name != "out"
        or out_package is None
        or out_package[0] != package
        or out_dir.parent.parent != script_dir.parent
        or not out_dir.is_relative_to(target)
    ):
        raise TraceError("Unknown build script unit layout")
    if package != cargo_package:
        raise TraceError(f"The target package {package} does not match CARGO_PKG_NAME {cargo_package}")
    return profile / ".fingerprint" / out_dir.parent.name


def fingerprint_names(fingerprint: Path, prefix: str) -> bool:
    if not fingerprint.is_dir():
        return False
    return any(entry.name.startswith(prefix) for entry in fingerprint.iterdir())


def has_unit_id_child(path: Path) -> bool:
    """If a direct child is a 16-hex unit directory, return true."""
    return any(child.is_dir() and UNIT_ID.fullmatch(child.name) for child in path.iterdir())


def is_unit_out_directory(path: Path) -> bool:
    """Return true for one Cargo unit `out` directory.

    A package name can end in 16 hex characters.
    A 16-hex child makes that directory a 1.100 package, not a 1.99 output directory.
    """
    if path.name != "out":
        return False
    parent = path.parent
    if parent.parent.name == "build" and classic_package_dir(parent.name) is not None:
        return not has_unit_id_child(parent)
    return bool(UNIT_ID.fullmatch(parent.name) and parent.parent.parent.name == "build")


def inside_unit_out(root: Path, path: Path) -> bool:
    """If `path` is strictly inside a unit `out` directory under `root`, return true."""
    if not path.is_relative_to(root) or path == root:
        return False
    current = path.parent
    while current.is_relative_to(root) and current != root:
        if is_unit_out_directory(current):
            return True
        if current.parent == current:
            break
        current = current.parent
    return False


def fingerprint_ready(path: Path, run: bool) -> None:
    """If `path` cannot be the claimed unit, raise TraceError.

    Cargo creates the fingerprint directory before it starts rustc or the script.
    Cargo writes the hash file and the `run-build-script` file after that process exits.
    A missing directory is an error.
    A directory that holds a marker of the other kind is an error.
    """
    reason = "Unknown build script unit layout" if run else "Unknown rustc unit layout"
    if not path.is_dir():
        raise TraceError(reason)
    for entry in path.iterdir():
        name = entry.name
        if name in ("invoked.timestamp", "jbotci-inputs-v1.json") or name.startswith(("dep-", "output-")):
            continue
        if run and not name.startswith("run-build-script-"):
            raise TraceError(reason)
        if not run and name.startswith("run-build-script-"):
            raise TraceError(reason)


def build_unit_build_script_fingerprint(target: Path, executable: Path, out_dir: Path,
                                         cargo_package: str) -> Path | None:
    """Return the build-unit run fingerprint for one build-script executable.

    Cargo 1.100 runs `<profile>/build/<package>/<compile id>/out/build_script_<stem>`.
    That compile unit's fingerprint holds a `build-script-*` file.
    `OUT_DIR` is `<profile>/build/<package>/<run id>/out` for a different id.
    The package name in that path must equal CARGO_PKG_NAME.
    Return None when `executable` is not in that place.
    """
    if not executable.name.startswith("build_script_") or executable.name == "build_script_":
        return None
    if executable.parent.name != "out":
        return None
    compile_dir = executable.parent.parent
    if not UNIT_ID.fullmatch(compile_dir.name):
        return None
    package_dir = compile_dir.parent
    build_dir = package_dir.parent
    # The package name can end in 16 hex characters. The `out/<id>` shape is the layout.
    if build_dir.name != "build":
        return None
    profile = build_dir.parent
    if not profile_inside_target(target, profile):
        raise TraceError("Unknown build script unit layout")
    compile_fingerprint = compile_dir / "fingerprint"
    if not compile_fingerprint.exists():
        return None
    if not compile_fingerprint.is_dir():
        raise TraceError("Unknown build script unit layout")
    if not fingerprint_names(compile_fingerprint, "build-script-"):
        return None
    run_id = out_dir.parent.name
    if (
        out_dir.name != "out"
        or not UNIT_ID.fullmatch(run_id)
        or run_id == compile_dir.name
        or out_dir.parent.parent != package_dir
        or not out_dir.is_relative_to(target)
    ):
        raise TraceError("Unknown build script unit layout")
    if package_dir.name != cargo_package:
        raise TraceError(
            f"The target package {package_dir.name} does not match CARGO_PKG_NAME {cargo_package}")
    return out_dir.parent / "fingerprint"


class Parser:
    def __init__(self, workspace: Path, target: Path, output,
                 inspect_process: Callable[[int], dict] | None = None):
        self.workspace = workspace
        self.target = target
        self.output = output
        self.processes: dict[int, Process] = {}
        self.pending: dict[int, str] = {}
        self.fork_states: dict[int, Process] = {}
        self.waiting: dict[int, list[str]] = {}
        self.identities: dict[int, dict] = {}
        self.interrupted_forks: list[dict] = []
        self.inspect_process = inspect_process
        self.errors = 0

    def emit(self, event: dict) -> None:
        encoded = json.dumps(event, ensure_ascii=True)
        self.output.write(encoded + "\n")
        if event["event"] in ("trace-error", "incomplete-processes"):
            print(f"Input trace error: {encoded}", file=sys.stderr)

    def consume(self, line: str) -> None:
        match = LINE.fullmatch(line)
        if not match:
            return
        pid, name, body = int(match[1]), match[2], match[3]
        identity = self.identities.setdefault(pid, {"exe": None, "cmdline": None, "parent": None})
        if name:
            identity["comm"] = name
        if pid not in self.processes:
            if not self.processes:
                self.processes[pid] = Process(str(self.workspace))
            else:
                if pid not in self.waiting and self.inspect_process is not None:
                    identity.update(self.inspect_process(pid))
                    self.emit({"event": "process", "pid": pid, **identity})
                if body.startswith("execve(") and body.endswith("= 0"):
                    executable, end = string_at(body)
                    arguments, _ = array_at(body, end)
                    identity.update(exe=executable, cmdline={"arguments": "<redacted>",
                                                           "argument_count": max(0, len(arguments) - 1)})
                    self.emit({"event": "process", "pid": pid, **identity})
                if EXIT.fullmatch(body) and all(
                    TEARDOWN.fullmatch(LINE.fullmatch(buffered)[3])
                    for buffered in self.waiting.get(pid, [])
                ):
                    # A dying thread can have no reported clone result. Retain the
                    # proof that every observed event is teardown, with no exec,
                    # directory change, or file open. Any other event stays queued.
                    self.emit({"event": "unattributed-exit", "pid": pid, **identity,
                               "teardown_events": len(self.waiting.get(pid, [])),
                               "observed_teardown": [LINE.fullmatch(buffered)[3]
                                                     for buffered in self.waiting.get(pid, [])],
                               "termination": body})
                    self.waiting.pop(pid, None)
                    return
                self.waiting.setdefault(pid, []).append(line)
                return
        if EXIT.fullmatch(body):
            # strace can show an unfinished syscall when a thread exits.
            # An explicit exit proves that the operation cannot return or read.
            self.pending.pop(pid, None)
            fork = self.fork_states.pop(pid, None)
            if fork is not None:
                event = {"event": "interrupted-fork", "pid": pid, **fork.identity(),
                         "comm": identity.get("comm")}
                self.interrupted_forks.append(event)
                self.emit(event)
            return
        if "<unfinished ...>" in body:
            self.pending[pid] = body.split("<unfinished ...>", 1)[0]
            if re.match(r"(?:clone3?|fork|vfork)\(", body):
                self.fork_states[pid] = replace(self.processes[pid])
            return
        if body.startswith("<... "):
            resumed = re.match(r"<\.\.\. \w+ resumed>(.*)", body)
            if resumed:
                body = self.pending.pop(pid, "") + resumed[1]
            else:
                return
        process = self.processes.setdefault(pid, Process(str(self.workspace)))
        if re.match(r"(?:clone3?|fork|vfork)\(", body):
            child = re.search(r"= (\d+)(?:<[^>]*>)?\s*$", body)
            if child:
                child_pid = int(child[1])
                self.processes[child_pid] = replace(self.fork_states.pop(pid, process), parent=pid)
                self.identities.setdefault(child_pid, {}).update(self.processes[child_pid].identity(), source="journal")
                self.emit({"event": "process", "pid": child_pid, **self.identities[child_pid]})
                for buffered in self.waiting.pop(child_pid, []):
                    self.consume(buffered)
            return
        if body.startswith("chdir(") and body.endswith("= 0"):
            directory, _ = string_at(body)
            process.cwd = str(Path(process.cwd) / directory)
            return
        if body.startswith("execve(") and body.endswith("= 0"):
            executable, end = string_at(body)
            arguments, end = array_at(body, end)
            environment, _ = array_at(body, end)
            process.exe = executable
            process.argument_count = max(0, len(arguments) - 1)
            identity.update(process.identity(), source="journal")
            self.emit({"event": "process", "pid": pid, **identity})
            # Never retain the raw environment or arguments in the journal.
            env = dict(item.split("=", 1) for item in environment if "=" in item)
            if Path(executable).name == "dx":
                process.via_dx = True
            package = env.get("CARGO_PKG_NAME")
            manifest = env.get("CARGO_MANIFEST_DIR")
            out = env.get("OUT_DIR")
            unit = None
            kind = None
            dep_info = None
            # Match a build script by its place in the target. A custom
            # `package.build` stem still uses that place. A normal binary
            # named build-script-build does not.
            resolved = normalized_path(executable, process.cwd)
            if package and manifest and out and resolved.is_relative_to(self.target):
                out_dir = normalized_path(out, process.cwd)
                # A nested Cargo target lives inside the outer run unit's OUT_DIR.
                # The 1.100 build-script executable also sits in a compile unit `out`
                # directory, so the executable path alone is not the nested check.
                if inside_unit_out(self.target, out_dir):
                    if process.unit is None:
                        raise TraceError("A nested build has no outer unit")
                else:
                    fingerprint = classic_build_script_fingerprint(self.target, resolved, out_dir, package)
                    if fingerprint is None:
                        fingerprint = build_unit_build_script_fingerprint(self.target, resolved, out_dir, package)
                    if fingerprint is not None:
                        if inside_unit_out(self.target, fingerprint):
                            if process.unit is None:
                                raise TraceError("A nested build has no outer unit")
                        elif not fingerprint.is_relative_to(self.target):
                            raise TraceError("Unknown build script unit layout")
                        else:
                            fingerprint_ready(fingerprint, run=True)
                            unit = fingerprint
                            kind = "run"
            # A rustc probe inherits Cargo's environment and can write into OUT_DIR.
            # Only Cargo's own metadata hash makes the call a unit.
            if package and manifest and Path(executable).name in ("rustc", "rustc.exe") and cargo_rustc_metadata(arguments):
                destination = flag(arguments, "--out-dir")
                suffix = extra_filename(arguments)
                crate = flag(arguments, "--crate-name")
                hashed_suffix = bool(suffix and UNIT_ID.fullmatch(suffix))
                if not destination or not crate:
                    raise TraceError("Unknown rustc unit layout")
                directory = normalized_path(destination, process.cwd)
                if inside_unit_out(self.target, directory):
                    if process.unit is None:
                        raise TraceError("A nested build has no outer unit")
                else:
                    fingerprint = build_unit_fingerprint(directory)
                    if fingerprint is not None:
                        path_package = directory.parents[1].name
                        if path_package != package:
                            raise TraceError(
                                f"The target package {path_package} does not match CARGO_PKG_NAME {package}")
                        if not profile_inside_target(self.target, fingerprint.parents[3]):
                            raise TraceError("Unknown rustc unit layout")
                        relative = fingerprint.relative_to(self.target).as_posix() if fingerprint.is_relative_to(self.target) else None
                        if relative is None or (suffix is not None and not hashed_suffix) or process.unit == relative:
                            raise TraceError("Unknown rustc unit layout")
                        fingerprint_ready(fingerprint, run=False)
                        unit = fingerprint
                        kind = "compile"
                        dep_name = f"{crate}-{suffix}.d" if hashed_suffix else f"{crate}.d"
                        dep_info = str(directory / dep_name)
                    elif hashed_suffix:
                        fingerprint = classic_fingerprint(self.target, directory, package, suffix)
                        if fingerprint is not None and inside_unit_out(self.target, fingerprint):
                            if process.unit is None:
                                raise TraceError("A nested build has no outer unit")
                        elif fingerprint is None or not fingerprint.is_relative_to(self.target):
                            raise TraceError("Unknown rustc unit layout")
                        else:
                            fingerprint_ready(fingerprint, run=False)
                            unit = fingerprint
                            kind = "compile"
                            dep_info = str(directory / f"{crate}-{suffix}.d")
                    else:
                        raise TraceError("Unknown rustc unit layout")
            if unit is not None:
                process.unit = unit.relative_to(self.target).as_posix()
                self.emit({"event": "unit", "unit": process.unit, "kind": kind,
                           "package": manifest, "cwd": process.cwd, "dep_info": dep_info,
                           "via_dx": process.via_dx})
            return
        if not body.startswith(("open(", "openat(", "openat2(")):
            return
        result = re.search(r"= (\d+)<(.*)>\s*$", body)
        if not result:
            return
        _, filename_end = string_at(body)
        flags = body[filename_end:result.start()]
        if re.search(r"\bO_DIRECTORY\b", flags):
            return
        access = re.search(r"\bO_(RDONLY|WRONLY|RDWR)\b", flags)
        if access is None:
            raise TraceError("Unknown file access mode in an input trace")
        opened = result[2]
        if opened.endswith(" (deleted)"):
            opened = opened[:-10]
        if not opened.startswith("/"):
            return
        if Path(opened).is_relative_to(self.target):
            return
        writing = access[1] in ("WRONLY", "RDWR")
        if process.unit is None and not writing:
            return
        self.emit({"event": "open", "unit": process.unit, "path": opened,
                   "write": writing, "read": access[1] != "WRONLY"})

    def feed(self, stream) -> None:
        for raw in stream:
            try:
                self.consume(raw.decode("utf-8", errors="surrogateescape").rstrip("\n"))
            except (TraceError, ValueError, SyntaxError, IndexError, KeyError, OSError) as error:
                # A raw line can contain credentials. Record no part of it.
                self.errors += 1
                match = re.match(rb"\s*(\d+)(?:<[^>]*>)?\s+(\w+)\(", raw)
                self.emit({"event": "trace-error", "pid": int(match[1]) if match else None,
                           "operation": match[2].decode("ascii") if match else "resumed",
                           "unit": self.processes.get(int(match[1]), Process(str(self.workspace))).unit if match else None,
                           "reason": str(error) if isinstance(error, TraceError) else type(error).__name__})
        if self.pending or self.waiting:
            self.errors += len(self.pending) + len(self.waiting)
            self.emit({"event": "incomplete-processes",
                       "pending": {pid: {"unit": self.processes[pid].unit,
                                         "operation": operation_name(body)} for pid, body in self.pending.items()},
                       "waiting": list(self.waiting),
                       "waiting_processes": {pid: self.identities[pid] for pid in self.waiting},
                       "interrupted_forks": self.interrupted_forks})
        self.emit({"event": "complete", "errors": self.errors})


def trace(workspace: Path, target: Path, journal: Path, command: list[str]) -> int:
    read_fd, write_fd = os.pipe()
    journal.parent.mkdir(parents=True, exist_ok=True)
    with journal.open("w") as output, os.fdopen(read_fd, "rb") as stream:
        parser = Parser(workspace, target, output, inspect_process=process_snapshot)
        reader = threading.Thread(target=parser.feed, args=(stream,))
        reader.start()
        try:
            process = subprocess.Popen([
                "strace", "-f", "--seccomp-bpf", "-q", "-v", "-s", "1048576", "-yy", "-Y",
                "-e", "trace=execve,clone,clone3,fork,vfork,chdir,open,openat,openat2",
                "-o", f"/dev/fd/{write_fd}", "--", *command,
            ], cwd=workspace, pass_fds=(write_fd,))
        finally:
            os.close(write_fd)
        status = process.wait()
        reader.join()
    if parser.errors:
        print(f"Input trace rejected {parser.errors} operations. Read the error events in {journal}.", file=sys.stderr)
    return status if status else (1 if parser.errors else 0)
